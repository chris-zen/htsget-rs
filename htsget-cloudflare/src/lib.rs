//! Library providing the routing and http responses for Cloudflare worker requests.
//!

use worker::*;
mod utils;

use std::collections::HashMap;
use std::sync::Arc;

use http::Uri;

use htsget_config::config::cors::CorsConfig;
pub use htsget_config::config::{Config, DataServerConfig, ServiceInfo, TicketServerConfig};
pub use htsget_config::regex_resolver::StorageType;
use htsget_http::{Endpoint, PostRequest};
use htsget_search::htsget::HtsGet;
// TODO: Implement this on CF workers
//use htsget_search::storage::configure_cors;

use crate::handlers::get::get;
use crate::handlers::post::post;
use crate::handlers::service_info::get_service_info_json;

pub mod handlers;

pub struct WorkerResponse(worker::Response);


impl TryFrom<http::Response<ResponseBody>> for WorkerResponse {
    type Error = Error;

    fn try_from(http_response: http::Response<ResponseBody>) -> std::result::Result<Self, Self::Error> {
        let (parts, body) = http_response.into_parts();

        // let body_stream = ReadableStream::from_stream(BodyStream::new(body));
        // let resp_body = ResponseBody::Stream(body);

        let resp = worker::Response::from_body(body).map_err(|err| err)?
            .with_headers(parts.headers.into())
            .with_status(parts.status.as_u16());

        Ok(WorkerResponse(resp))
    }
}

fn log_request(req: &Request) {
  console_log!(
    "{} - [{}], located at: {:?}, within: {}",
    Date::now().to_string(),
    req.path(),
    req.cf().coordinates().unwrap_or_default(),
    req.cf().region().unwrap_or_else(|| "unknown region".into())
  );
}

/// A request route, with a method, endpoint and route type.
#[derive(Debug, PartialEq, Eq)]
pub struct Route {
  method: HtsgetMethod,
  endpoint: Endpoint,
  route_type: RouteType,
}

/// Valid htsget http request methods.
#[derive(Debug, PartialEq, Eq)]
pub enum HtsgetMethod {
  Get,
  Post,
}

/// A route type, which is either the service info endpoint, or an id represented by a string.
#[derive(Debug, PartialEq, Eq)]
pub enum RouteType {
  ServiceInfo,
  Id(String),
}

impl Route {
  pub fn new(method: HtsgetMethod, endpoint: Endpoint, route_type: RouteType) -> Self {
    Self {
      method,
      endpoint,
      route_type,
    }
  }
}

/// A Router is a struct which handles routing any htsget requests to the htsget search, using the config.
pub struct Router<'a, H> {
  searcher: Arc<H>,
  config_service_info: &'a ServiceInfo,
}

impl<'a, H: HtsGet + Send + Sync + 'static> Router<'a, H> {
  pub fn new(searcher: Arc<H>, config_service_info: &'a ServiceInfo) -> Self {
    Self {
      searcher,
      config_service_info,
    }
  }

  /// Gets the Route if the request is valid, otherwise returns None.
  fn get_route(&self, method: &Method, uri: &Uri) -> Option<Route> {
    let with_endpoint = |endpoint: Endpoint, endpoint_type: &str| {
      if endpoint_type.is_empty() {
        None
      } else {
        let method = match *method {
          Method::Get => Some(HtsgetMethod::Get),
          Method::Post => Some(HtsgetMethod::Post),
          _ => None,
        }?;
        if endpoint_type == "service-info" {
          Some(Route::new(method, endpoint, RouteType::ServiceInfo))
        } else {
          Some(Route::new(
            method,
            endpoint,
            RouteType::Id(endpoint_type.to_string()),
          ))
        }
      }
    };

    uri.path().strip_prefix("/reads/").map_or_else(
      || {
        uri
          .path()
          .strip_prefix("/variants/")
          .and_then(|variants| with_endpoint(Endpoint::Variants, variants))
      },
      |reads| with_endpoint(Endpoint::Reads, reads),
    )
  }

  /// Routes the request to the relevant htsget search endpoint using the (Cloudflare) worker request, returning a http response.
  pub async fn route_request(&self, request: Request) -> http::Result<Response> {
    match self.get_route(&request.method(), &request.path().parse::<Uri>()?) {
      Some(Route {
        endpoint,
        route_type: RouteType::ServiceInfo,
        ..
      }) => {
        let response = get_service_info_json(self.searcher.clone(), endpoint, self.config_service_info)?;
        WorkerResponse::try_from(response).map_err(|err| err.into()).map(|value| value.0)
      },
      Some(Route {
        method: HtsgetMethod::Get,
        endpoint,
        route_type: RouteType::Id(id),
      }) => {
        get(
          id,
          self.searcher.clone(),
          Self::extract_query(&request),
          endpoint,
        ).await.into()
      }
      Some(Route {
        method: HtsgetMethod::Post,
        endpoint,
        route_type: RouteType::Id(id),
      }) => match Self::extract_query_from_payload(&mut request).await {
        None => Ok(
          http::Response::builder()
            .status(http::StatusCode::UNSUPPORTED_MEDIA_TYPE)
            .body(ResponseBody::Empty),
        ),
        Some(query) => post(id, self.searcher.clone(), query, endpoint).await,
      },
      _ => Ok(
        http::Response::builder()
          .status(http::StatusCode::METHOD_NOT_ALLOWED)
          .body(ResponseBody::Empty).into()?,
      ),
    }
  }

  /// Extracts post request query parameters.
  async fn extract_query_from_payload(request: &mut Request) -> Option<PostRequest> {
    if request.bytes().await.unwrap().is_empty() {
      Some(PostRequest::default())
    } else {
      let payload = request.json().await;
      payload.ok()
    }
  }

  /// Extract get request query parameters.
  fn extract_query(request: &Request) -> HashMap<String, String> {
    let mut query = HashMap::new();
    for (key, value) in request.url().unwrap().query_pairs() {
      query.insert(key.to_string(), value.to_string());
    }
    query
  }
}

pub async fn handle_request<H>(cors: CorsConfig, router: &Router<'_, H>) -> Result<()>
where
  H: HtsGet + Send + Sync + 'static,
{
  // Optionally, get more helpful error messages written to the console in the case of a panic.
  utils::set_panic_hook();

  unimplemented!();
  
  //Ok(())
}