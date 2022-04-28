//! The following module provides an implementation of [UrlFormatter] for https, and the server
//! code which responds to formatted urls.
//!
//! This is the code that replies to the url tickets generated by [HtsGet], in the case of [LocalStorage].
//!

use std::fs::File;
use std::io::BufReader;
use std::net::{AddrParseError, SocketAddr};
use std::path::Path;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

use axum::http;
use axum::Router;
use axum_extra::routing::SpaRouter;
use futures_util::future::poll_fn;
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, Http};
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio::net::TcpListener;
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};
use tokio_rustls::TlsAcceptor;
use tower::MakeService;

use crate::storage::StorageError::ResponseServerError;
use crate::storage::UrlFormatter;

use super::{Result, StorageError};

/// Https url formatter.
#[derive(Debug, Clone)]
pub struct HttpsFormatter {
  addr: SocketAddr,
}

impl HttpsFormatter {
  pub fn new(ip: impl Into<String>, port: impl Into<String>) -> Result<Self> {
    Ok(Self {
      addr: SocketAddr::from_str(&format!("{}:{}", ip.into(), port.into()))?,
    })
  }

  /// Eagerly bind the address by returing an AxumStorageServer.
  pub async fn bind_axum_server(&self) -> Result<AxumStorageServer> {
    AxumStorageServer::bind_addr(&self.addr).await
  }
}

impl From<AddrParseError> for StorageError {
  fn from(err: AddrParseError) -> Self {
    StorageError::InvalidAddress(err)
  }
}

impl From<SocketAddr> for HttpsFormatter {
  fn from(addr: SocketAddr) -> Self {
    Self { addr }
  }
}

/// The local storage static http server.
#[derive(Debug)]
pub struct AxumStorageServer {
  listener: AddrIncoming,
}

impl AxumStorageServer {
  const SERVE_ASSETS_AT: &'static str = "/data";

  /// Eagerly bind the the address for use with the server, returning any errors.
  pub async fn bind_addr(addr: &SocketAddr) -> Result<Self> {
    let listener = TcpListener::bind(addr).await?;
    let listener = AddrIncoming::from_listener(listener)?;
    Ok(Self { listener })
  }

  /// Run the actual server, using the provided path, key and certificate.
  pub async fn serve<P: AsRef<Path>>(&mut self, path: P, key: P, cert: P) -> Result<()> {
    let mut app = Router::new()
      .merge(SpaRouter::new(Self::SERVE_ASSETS_AT, path))
      .into_make_service_with_connect_info::<SocketAddr>();

    let rustls_config = Self::rustls_server_config(key, cert)?;
    let acceptor = TlsAcceptor::from(rustls_config);

    loop {
      let stream = poll_fn(|cx| Pin::new(&mut self.listener).poll_accept(cx))
        .await
        .ok_or_else(|| ResponseServerError("Poll accept failed.".to_string()))?
        .map_err(|err| ResponseServerError(err.to_string()))?;
      let acceptor = acceptor.clone();

      let app = app
        .make_service(&stream)
        .await
        .map_err(|err| ResponseServerError(err.to_string()))?;

      tokio::spawn(async move {
        if let Ok(stream) = acceptor.accept(stream).await {
          let _ = Http::new().serve_connection(stream, app).await;
        }
      });
    }
  }

  fn rustls_server_config<P: AsRef<Path>>(key: P, cert: P) -> Result<Arc<ServerConfig>> {
    let mut key_reader = BufReader::new(File::open(key)?);
    let mut cert_reader = BufReader::new(File::open(cert)?);

    let key = PrivateKey(pkcs8_private_keys(&mut key_reader)?.remove(0));
    let certs = certs(&mut cert_reader)?
      .into_iter()
      .map(Certificate)
      .collect();

    let mut config = ServerConfig::builder()
      .with_safe_defaults()
      .with_no_client_auth()
      .with_single_cert(certs, key)
      .map_err(|err| ResponseServerError(err.to_string()))?;

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(Arc::new(config))
  }
}

impl From<hyper::Error> for StorageError {
  fn from(error: hyper::Error) -> Self {
    ResponseServerError(error.to_string())
  }
}

impl UrlFormatter for HttpsFormatter {
  fn format_url(&self, path: String) -> Result<String> {
    http::uri::Builder::new()
      .scheme(http::uri::Scheme::HTTPS)
      .authority(self.addr.to_string())
      .path_and_query(path)
      .build()
      .map_err(|err| StorageError::InvalidUri(err.to_string()))
      .map(|value| value.to_string())
  }
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::io::Read;

  use http::{Method, Request};
  use hyper::client::HttpConnector;
  use hyper::{Body, Client};
  use hyper_tls::native_tls::TlsConnector;
  use hyper_tls::HttpsConnector;
  use rcgen::generate_simple_self_signed;

  use crate::storage::local::tests::create_local_test_files;

  use super::*;

  #[tokio::test]
  async fn test_server() {
    let (_, base_path) = create_local_test_files().await;
    let key_path = base_path.path().join("key.pem");
    let cert_path = base_path.path().join("cert.pem");

    // Generate self-signed certificate.
    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    fs::write(key_path.clone(), cert.serialize_private_key_pem()).unwrap();
    fs::write(cert_path.clone(), cert.serialize_pem().unwrap()).unwrap();

    // Read certificate.
    let mut buf = vec![];
    File::open(cert_path.clone())
      .unwrap()
      .read_to_end(&mut buf)
      .unwrap();
    let cert = hyper_tls::native_tls::Certificate::from_pem(&buf).unwrap();

    // Add self-signed certificate to connector.
    let tls = TlsConnector::builder()
      .add_root_certificate(cert)
      .build()
      .unwrap();
    let mut http = HttpConnector::new();
    http.enforce_http(false);
    let https = HttpsConnector::from((http, tls.into()));

    // Start server.
    let addr = SocketAddr::from_str(&format!("{}:{}", "127.0.0.1", "8080")).unwrap();
    let mut server = AxumStorageServer::bind_addr(&addr).await.unwrap();
    tokio::spawn(async move { server.serve(base_path.path(), &key_path, &cert_path).await.unwrap() });

    // Make request.
    let client = Client::builder().build::<_, hyper::Body>(https);
    let request = Request::builder()
      .method(Method::GET)
      .uri(format!("https://{}:{}/data/key1", "localhost", "8080"))
      .body(Body::empty())
      .unwrap();
    let response = client.request(request).await;

    let body = hyper::body::to_bytes(response.unwrap().into_body())
      .await
      .unwrap();
    assert_eq!(body.as_ref(), b"value1");
  }

  #[test]
  fn https_formatter_format_authority() {
    let formatter = HttpsFormatter::new("127.0.0.1", "8080").unwrap();
    assert_eq!(formatter.format_url("/path".to_string()).unwrap(), "https://127.0.0.1:8080/path")
  }
}
