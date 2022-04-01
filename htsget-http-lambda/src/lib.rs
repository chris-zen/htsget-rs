pub mod handlers;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use lambda_http::{Body, IntoResponse, Request, Response, Error};
use lambda_http::http::{Method, StatusCode, Uri};
use htsget_config::config::HtsgetConfig;
use regex::Regex;
use lambda_http::ext::RequestExt;
use lambda_http::http::header::CONTENT_TYPE;
use serde::de::DeserializeOwned;
use htsget_http_core::{Endpoint, PostRequest};
use htsget_search::htsget::HtsGet;
use crate::handlers::service_info::get_service_info_json;
use crate::handlers::get::get;
use crate::handlers::post::post;

pub async fn lambda_function<H: HtsGet + Send + Sync + 'static>(request: Request, searcher: Arc<H>, config: &HtsgetConfig, route_matcher: RouteMatcher) -> Response<Body> {
  match route_matcher.get_route(request.method(), request.uri()) {
    Some(Route { method: _, endpoint, route_type: RouteType::ServiceInfo }) => {
      get_service_info_json(searcher, endpoint, config).into_response()
    },
    Some(Route { method: HtsgetMethod::Get, endpoint, route_type: RouteType::Id(id) }) => {
      get(id, searcher, extract_query(&request), endpoint).await.into_response()
    },
    Some(Route { method: HtsgetMethod::Post, endpoint, route_type: RouteType::Id(id) }) => {
      match extract_query_from_payload(&request) {
        None => Response::builder().status(StatusCode::UNSUPPORTED_MEDIA_TYPE).body("").unwrap().into_response(),
        Some(query) => post(id, searcher, query, endpoint).await.into_response()
      }
    },
    _ => Response::builder().status(StatusCode::METHOD_NOT_ALLOWED).body("").unwrap().into_response()
  }
}

fn extract_query_from_payload(request: &Request) -> Option<PostRequest> {
  // Check if the content type is application/json
  let content_type = request.headers().get(CONTENT_TYPE)?;
  if content_type.to_str().ok()? != mime::APPLICATION_JSON.as_ref() {
    return None;
  }

  request.payload().ok()?
}

/// Extract a query hashmap from a request.
fn extract_query(request: &Request) -> HashMap<String, String> {
  let mut query = HashMap::new();
  // Silently ignores all but the last query key, for keys that are present more than once.
  // This is the way actix-web does it, but should we return an error instead if a key is present
  // more than once?
  for (key, value) in request.query_string_parameters().iter() {
    query.insert(key.to_string(), value.to_string());
  }
  query
}

#[derive(Debug, PartialEq)]
pub struct Route {
  method: HtsgetMethod,
  endpoint: Endpoint,
  route_type: RouteType
}

#[derive(Debug, PartialEq)]
pub enum HtsgetMethod {
  Get,
  Post
}

#[derive(Debug, PartialEq)]
pub enum RouteType {
  ServiceInfo,
  Id(String)
}

impl Route {
  pub fn new(method: HtsgetMethod, endpoint: Endpoint, route_type: RouteType) -> Self {
    Self { method, endpoint, route_type }
  }
}

pub struct RouteMatcher {
  regex: Regex
}

impl RouteMatcher {
  const ENDPOINT_CAPTURE_NAME: &'static str = "endpoint";
  const SERVICE_INFO_CAPTURE_NAME: &'static str = "service_info";
  const ID_CAPTURE_NAME: &'static str = "id";

  pub fn new() -> Self {
    Self { regex: Self::regex_path() }
  }

  pub fn get_route(&self, method: &Method, uri: &Uri) -> Option<Route> {
    let captures = self.regex.captures(uri.path())?;
    let endpoint: Endpoint = Endpoint::from_str(captures.name(Self::ENDPOINT_CAPTURE_NAME)?.as_str()).expect("Expected valid endpoint.");
    let method = match *method {
      Method::GET => Some(HtsgetMethod::Get),
      Method::POST => Some(HtsgetMethod::Post),
      _ => None
    }?;

    if captures.name(Self::SERVICE_INFO_CAPTURE_NAME).is_some() {
      Some(Route::new(method, endpoint, RouteType::ServiceInfo))
    } else {
      Some(Route::new(method, endpoint, RouteType::Id(captures.name(Self::ID_CAPTURE_NAME)?.as_str().to_string())))
    }
  }

  /// Regex which matches the relevant parts of a htsget uri path.
  fn regex_path() -> Regex {
    let pattern= format!(r"^/(?P<{}>reads|variants)/(?:(?P<{}>service-info$)|(?P<{}>.+$))", Self::ENDPOINT_CAPTURE_NAME, Self::SERVICE_INFO_CAPTURE_NAME, Self::ID_CAPTURE_NAME);
    Regex::new(&pattern).expect("Expected valid regex pattern.")
  }
}

#[cfg(test)]
mod tests {
  use lambda_http::http::Uri;
  use lambda_http::Request;
  use htsget_http_core::Endpoint;
  use crate::{Body, HtsgetMethod, Method, Route, RouteMatcher, RouteType};

  #[test]
  fn test_route_matcher_invalid_method() {
    let route_matcher = RouteMatcher::new();
    let uri = Uri::builder().path_and_query("/reads/id").build().unwrap();
    assert!(route_matcher.get_route(&Method::DELETE, &uri).is_none());
  }

  #[test]
  fn test_route_matcher_no_endpoint() {
    let route_matcher = RouteMatcher::new();
    let uri = Uri::builder().path_and_query("/path/").build().unwrap();
    assert!(route_matcher.get_route(&Method::GET, &uri).is_none());
  }

  #[test]
  fn test_route_matcher_reads_no_id() {
    let route_matcher = RouteMatcher::new();
    let uri = Uri::builder().path_and_query("/reads/").build().unwrap();
    assert!(route_matcher.get_route(&Method::GET, &uri).is_none());
  }

  #[test]
  fn test_route_matcher_variants_no_id() {
    let route_matcher = RouteMatcher::new();
    let uri = Uri::builder().path_and_query("/variants/").build().unwrap();
    assert!(route_matcher.get_route(&Method::GET, &uri).is_none());
  }

  #[test]
  fn test_route_matcher_reads_service_info() {
    let route_matcher = RouteMatcher::new();
    let uri = Uri::builder().path_and_query("/reads/service-info").build().unwrap();
    let route = route_matcher.get_route(&Method::GET, &uri);
    assert_eq!(route, Some(Route { method: HtsgetMethod::Get, endpoint: Endpoint::Reads, route_type: RouteType::ServiceInfo }));
  }

  #[test]
  fn test_route_matcher_variants_service_info() {
    let route_matcher = RouteMatcher::new();
    let uri = Uri::builder().path_and_query("/variants/service-info").build().unwrap();
    let route = route_matcher.get_route(&Method::GET, &uri);
    assert_eq!(route, Some(Route { method: HtsgetMethod::Get, endpoint: Endpoint::Variants, route_type: RouteType::ServiceInfo }));
  }

  #[test]
  fn test_route_matcher_reads_id() {
    let route_matcher = RouteMatcher::new();
    let uri = Uri::builder().path_and_query("/reads/id").build().unwrap();
    let route = route_matcher.get_route(&Method::GET, &uri);
    assert_eq!(route, Some(Route { method: HtsgetMethod::Get, endpoint: Endpoint::Reads, route_type: RouteType::Id("id".to_string()) }));
  }

  #[test]
  fn test_route_matcher_variants_id() {
    let route_matcher = RouteMatcher::new();
    let uri = Uri::builder().path_and_query("/variants/id").build().unwrap();
    let route = route_matcher.get_route(&Method::GET, &uri);
    assert_eq!(route, Some(Route { method: HtsgetMethod::Get, endpoint: Endpoint::Variants, route_type: RouteType::Id("id".to_string()) }));
  }
}