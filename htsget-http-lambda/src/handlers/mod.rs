mod service_info;

use lambda_http::http::{header, StatusCode};
use lambda_http::IntoResponse;
use serde::Serialize;
use crate::{Body, Response};

pub use crate::handlers::service_info::get_service_info_json;

pub struct FormatJson<T>(pub T);

impl<T: Serialize> IntoResponse for FormatJson<T> {
  fn into_response(self) -> Response<Body> {
    let mut body = match serde_json::to_string_pretty(&self.0) {
      Ok(body) => body,
      Err(e) => return from_error(e),
    };
    body.push('\n');

    Response::builder().status(StatusCode::OK).header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref()).body(body).expect("Expected valid response.").into_response()
  }
}

fn from_error(error: serde_json::Error) -> Response<Body> {
  // This should be the same as actix-web.
  Response::builder()
    .status(StatusCode::INTERNAL_SERVER_ERROR)
    .header(header::CONTENT_TYPE, mime::TEXT_PLAIN_UTF_8.as_ref())
    .body(format!("{}", error)).expect("Expected valid response.").into_response()
}

#[cfg(test)]
mod tests {
  use lambda_http::http::{header, HeaderMap, StatusCode};
  use lambda_http::{IntoResponse};
  use serde::{Serialize, Serializer};
  use serde::ser::Error;
  use serde_json::json;
  use crate::handlers::FormatJson;

  struct TestError;

  impl Serialize for TestError {
    fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error> where S: Serializer {
      Err(S::Error::custom("err"))
    }
  }

  #[test]
  fn into_response() {
    let expected_body = json!({"value": "1"});
    let expected_status_code = StatusCode::OK;
    let mut expected_headers = HeaderMap::new();
    expected_headers.insert(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref().parse().unwrap());

    let json = FormatJson(expected_body.clone());
    let response = json.into_response();
    assert_eq!(response.status(), expected_status_code);
    assert_eq!(response.headers(), &expected_headers);

    let json_value = serde_json::to_value(response.body()).unwrap();
    let json_string = json_value.as_str().unwrap();
    assert_eq!(json_string, serde_json::to_string_pretty(&expected_body).unwrap() + "\n");
  }

  #[test]
  fn into_response_error() {
    let expected_status_code = StatusCode::INTERNAL_SERVER_ERROR;
    let mut expected_headers = HeaderMap::new();
    expected_headers.insert(header::CONTENT_TYPE, mime::TEXT_PLAIN_UTF_8.as_ref().parse().unwrap());

    let json = FormatJson(TestError);
    let response = json.into_response();
    assert_eq!(response.status(), expected_status_code);
    assert_eq!(response.headers(), &expected_headers);

    let json_value = serde_json::to_value(response.body()).unwrap();
    let json_string = json_value.as_str().unwrap();
    assert_eq!(json_string, "err");
  }
}