//! Module providing an implementation for the [Storage] trait using Amazon's S3 object storage service.
use std::io::Cursor;
use std::time::Duration;

use async_trait::async_trait;
use aws_sdk_s3::client::fluent_builders;
use aws_sdk_s3::model::StorageClass;
use aws_sdk_s3::output::HeadObjectOutput;
use aws_sdk_s3::presigning::config::PresigningConfig;
use aws_sdk_s3::Client;
use bytes::Bytes;
use fluent_builders::GetObject;
use tokio::io::BufReader;

use htsget_id_resolver::{HtsGetIdResolver, RegexResolver};

use crate::htsget::Url;
use crate::storage::async_storage::AsyncStorage;
use crate::storage::aws::Retrieval::{Delayed, Immediate};
use crate::storage::{BytesRange, StorageError};

use super::{GetOptions, Result, UrlOptions};

#[derive(Debug)]
pub enum Retrieval {
  Immediate(StorageClass),
  Delayed(StorageClass),
}

/// Implementation for the [Storage] trait utilising data from an S3 bucket.
pub struct AwsS3Storage {
  client: Client,
  bucket: String,
  id_resolver: RegexResolver,
}

impl AwsS3Storage {
  // Allow the user to set this?
  pub const PRESIGNED_REQUEST_EXPIRY: u64 = 1000;

  pub fn new(client: Client, bucket: String, id_resolver: RegexResolver) -> Self {
    AwsS3Storage {
      client,
      bucket,
      id_resolver,
    }
  }

  pub async fn new_with_default_config(bucket: String, id_resolver: RegexResolver) -> Self {
    AwsS3Storage::new(
      Client::new(&aws_config::load_from_env().await),
      bucket,
      id_resolver,
    )
  }

  fn resolve_key<K: AsRef<str> + Send>(&self, key: &K) -> Result<String> {
    self
      .id_resolver
      .resolve_id(key.as_ref())
      .ok_or_else(|| StorageError::InvalidKey(key.as_ref().to_string()))
  }

  async fn s3_presign_url<K: AsRef<str> + Send>(
    &self,
    key: K,
    range: BytesRange,
  ) -> Result<String> {
    let response = self
      .client
      .get_object()
      .bucket(&self.bucket)
      .key(&self.resolve_key(&key)?);
    let response = Self::apply_range(response, range);
    Ok(
      response
        .presigned(
          PresigningConfig::expires_in(Duration::from_secs(Self::PRESIGNED_REQUEST_EXPIRY))
            .map_err(|err| StorageError::AwsError(err.to_string(), key.as_ref().to_string()))?,
        )
        .await
        .map_err(|err| StorageError::AwsError(err.to_string(), key.as_ref().to_string()))?
        .uri()
        .to_string(),
    )
  }

  async fn s3_head<K: AsRef<str> + Send>(&self, key: K) -> Result<HeadObjectOutput> {
    Ok(
      self
        .client
        .head_object()
        .bucket(&self.bucket)
        .key(&self.resolve_key(&key)?)
        .send()
        .await
        .map_err(|err| StorageError::AwsError(err.to_string(), key.as_ref().to_string()))?,
    )
  }

  pub async fn get_retrieval_type<K: AsRef<str> + Send>(&self, key: K) -> Result<Retrieval> {
    let head = self.s3_head(&self.resolve_key(&key)?).await?;
    Ok(
      // Default is Standard.
      match head.storage_class.unwrap_or(StorageClass::Standard) {
        class @ (StorageClass::DeepArchive | StorageClass::Glacier) => {
          Self::check_restore_header(head.restore, class)
        }
        class @ StorageClass::IntelligentTiering => {
          if head.archive_status.is_some() {
            // Not sure if this check is necessary for the archived intelligent tiering classes but
            // it shouldn't hurt.
            Self::check_restore_header(head.restore, class)
          } else {
            Immediate(class)
          }
        }
        class => Immediate(class),
      },
    )
  }

  fn check_restore_header(restore_header: Option<String>, class: StorageClass) -> Retrieval {
    if let Some(restore) = restore_header {
      if restore.contains("ongoing-request=\"false\"") {
        return Immediate(class);
      }
    }
    Delayed(class)
  }

  fn apply_range(builder: GetObject, range: BytesRange) -> GetObject {
    let range: String = range.into();
    if range.is_empty() {
      builder
    } else {
      builder.range(range)
    }
  }

  async fn get_content<K: AsRef<str> + Send>(&self, key: K, options: GetOptions) -> Result<Bytes> {
    // It would be nice to use a ready-made type with a ByteStream that implements AsyncRead + AsyncSeek
    // in order to avoid reading the whole byte buffer into memory. A custom type could be made similar to
    // https://users.rust-lang.org/t/what-to-pin-when-implementing-asyncread/63019/2 which could be based off
    // StreamReader.
    let response = self
      .client
      .get_object()
      .bucket(&self.bucket)
      .key(&self.resolve_key(&key)?);
    let response = Self::apply_range(response, options.range);
    Ok(
      response
        .send()
        .await
        .map_err(|err| StorageError::AwsError(err.to_string(), key.as_ref().to_string()))?
        .body
        .collect()
        .await
        .map_err(|err| StorageError::AwsError(err.to_string(), key.as_ref().to_string()))?
        .into_bytes(),
    )
  }

  async fn create_buf_reader<K: AsRef<str> + Send>(
    &self,
    key: K,
    options: GetOptions,
  ) -> Result<BufReader<Cursor<Bytes>>> {
    let response = self.get_content(key, options).await?;
    let cursor = Cursor::new(response);
    let reader = tokio::io::BufReader::new(cursor);
    Ok(reader)
  }
}

#[async_trait]
impl AsyncStorage for AwsS3Storage {
  type Streamable = BufReader<Cursor<Bytes>>;

  async fn get<K: AsRef<str> + Send>(
    &self,
    key: K,
    options: GetOptions,
  ) -> Result<Self::Streamable> {
    let key = key.as_ref();
    self.create_buf_reader(key, options).await
  }

  /// Returns a S3-presigned htsget URL
  async fn url<K: AsRef<str> + Send>(&self, key: K, options: UrlOptions) -> Result<Url> {
    let key = key.as_ref();
    let presigned_url = self.s3_presign_url(key, options.range.clone()).await?;
    Ok(options.apply(Url::new(presigned_url)))
  }

  /// Returns the size of the S3 object in bytes.
  async fn head<K: AsRef<str> + Send>(&self, key: K) -> Result<u64> {
    let key = key.as_ref();
    let head = self.s3_head(key).await?;
    Ok(head.content_length as u64)
  }
}

#[cfg(test)]
mod tests {
  use std::future::Future;
  use std::net::TcpListener;
  use std::path::Path;

  use aws_sdk_s3::{Client, Endpoint};
  use aws_types::credentials::SharedCredentialsProvider;
  use aws_types::region::Region;
  use aws_types::{Credentials, SdkConfig};
  use futures::future;
  use http::Uri;
  use hyper::service::make_service_fn;
  use hyper::Server;
  use s3_server::storages::fs::FileSystem;
  use s3_server::{S3Service, SimpleAuth};

  use crate::htsget::Headers;
  use crate::storage::local::tests::create_local_test_files;

  use super::*;

  async fn with_s3_test_server<F, Fut>(server_base_path: &Path, test: F)
  where
    F: FnOnce(Client) -> Fut,
    Fut: Future<Output = ()>,
  {
    // Setup s3-server.
    let fs = FileSystem::new(server_base_path).unwrap();
    let mut auth = SimpleAuth::new();
    auth.register(String::from("access_key"), String::from("secret_key"));
    let mut service = S3Service::new(fs);
    service.set_auth(auth);

    // Spawn hyper Server instance.
    let service = service.into_shared();
    let listener = TcpListener::bind(("localhost", 8014)).unwrap();
    let make_service: _ =
      make_service_fn(move |_| future::ready(Ok::<_, anyhow::Error>(service.clone())));
    let handle = tokio::spawn(Server::from_tcp(listener).unwrap().serve(make_service));

    // Create S3Config.
    let config = SdkConfig::builder()
      .region(Region::new("ap-southeast-2"))
      .credentials_provider(SharedCredentialsProvider::new(Credentials::from_keys(
        "access_key",
        "secret_key",
        None,
      )))
      .build();
    let ep = Endpoint::immutable(Uri::from_static("http://localhost:8014"));
    let s3_conf = aws_sdk_s3::config::Builder::from(&config)
      .endpoint_resolver(ep)
      .build();

    test(Client::from_conf(s3_conf));

    handle.abort();
  }

  async fn with_aws_s3_storage<F, Fut>(test: F)
  where
    F: FnOnce(AwsS3Storage) -> Fut,
    Fut: Future<Output = ()>,
  {
    let (folder_name, base_path) = create_local_test_files().await;
    with_s3_test_server(base_path.path(), |client| async move {
      test(AwsS3Storage::new(
        client,
        folder_name,
        RegexResolver::new(".*", "$0").unwrap(),
      ));
    })
    .await;
  }

  #[tokio::test]
  async fn existing_key() {
    with_aws_s3_storage(|storage| async move {
      let result = storage.get("key2", GetOptions::default()).await;
      assert!(matches!(result, Ok(_)));
    })
    .await;
  }

  #[tokio::test]
  async fn non_existing_key() {
    with_aws_s3_storage(|storage| async move {
      let result = storage.get("non-existing-key", GetOptions::default()).await;
      assert!(matches!(result, Err(StorageError::AwsError(_, _))));
    })
    .await;
  }

  #[tokio::test]
  async fn url_of_non_existing_key() {
    with_aws_s3_storage(|storage| async move {
      let result = storage.url("non-existing-key", UrlOptions::default()).await;
      assert!(matches!(result, Err(StorageError::AwsError(_, _))));
    })
    .await;
  }

  #[tokio::test]
  async fn url_of_existing_key() {
    with_aws_s3_storage(|storage| async move {
      let result = storage.url("key2", UrlOptions::default()).await.unwrap();
      assert!(result
        .url
        .starts_with(&format!("http://localhost:8014/{}/{}", "folder", "key2")));
      assert!(result.url.contains(&format!(
        "Amz-Expires={}",
        AwsS3Storage::PRESIGNED_REQUEST_EXPIRY
      )));
    })
    .await;
  }

  #[tokio::test]
  async fn url_with_specified_range() {
    with_aws_s3_storage(|storage| async move {
      let result = storage
        .url(
          "key2",
          UrlOptions::default().with_range(BytesRange::new(Some(7), Some(9))),
        )
        .await
        .unwrap();
      assert!(result
        .url
        .starts_with(&format!("http://localhost:8014/{}/{}", "folder", "key2")));
      assert!(result.url.contains(&format!(
        "Amz-Expires={}",
        AwsS3Storage::PRESIGNED_REQUEST_EXPIRY
      )));
      assert!(result.url.contains("range"));
      assert_eq!(
        result.headers,
        Some(Headers::default().with_header("Range", "bytes=7-9"))
      );
    })
    .await;
  }

  #[tokio::test]
  async fn url_with_specified_open_ended_range() {
    with_aws_s3_storage(|storage| async move {
      let result = storage
        .url(
          "key2",
          UrlOptions::default().with_range(BytesRange::new(Some(7), None)),
        )
        .await
        .unwrap();
      assert!(result
        .url
        .starts_with(&format!("http://localhost:8014/{}/{}", "folder", "key2")));
      assert!(result.url.contains(&format!(
        "Amz-Expires={}",
        AwsS3Storage::PRESIGNED_REQUEST_EXPIRY
      )));
      assert!(result.url.contains("range"));
      assert_eq!(
        result.headers,
        Some(Headers::default().with_header("Range", "bytes=7-"))
      );
    })
    .await;
  }

  #[tokio::test]
  async fn file_size() {
    with_aws_s3_storage(|storage| async move {
      let result = storage.head("key2").await;
      let expected: u64 = 6;
      assert!(matches!(result, Ok(size) if size == expected));
    })
    .await;
  }

  #[tokio::test]
  async fn retrieval_type() {
    with_aws_s3_storage(|storage| async move {
      let result = storage.get_retrieval_type("key2").await;
      println!("{:?}", result);
    })
    .await;
  }
}
