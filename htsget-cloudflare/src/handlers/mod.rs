//! Module primarily providing http response functionality for the htsget endpoints.
//!

use http::{self, Response, StatusCode};
use serde::Serialize;
use serde_json;
use worker::{self, ResponseBody};

use htsget_http::{HtsGetError, Result};
use htsget_search::htsget::JsonResponse;

pub mod get;
pub mod post;
pub mod service_info;

/// New type used for formatting a http response.
pub struct FormatJson<T>(T);

impl<T> FormatJson<T> {
  pub fn into_inner(self) -> T {
    self.0
  }
}

// Ok HtsGet Response
impl<T: Serialize> TryFrom<FormatJson<T>> for Response<ResponseBody> {
  type Error = http::Error;

  fn try_from(value: FormatJson<T>) -> http::Result<Self> {
    let mut body = match serde_json::to_string_pretty(&value.into_inner()) {
      Ok(body) => body,
      Err(e) => return Ok(FormatJson::try_from(e)?.into_inner()),
    };
    body.push('\n');

    Response::builder()
      .status(StatusCode::OK)
      .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
      .body(ResponseBody::Body(body.into()))
  }
}

// Error HTTP response
impl TryFrom<serde_json::Error> for FormatJson<Response<ResponseBody>> {
  type Error = http::Error;

  fn try_from(error: serde_json::Error) -> http::Result<Self> {
    Ok(Self(
      Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(http::header::CONTENT_TYPE, mime::TEXT_PLAIN_UTF_8.as_ref())
        .body(ResponseBody::Body(error.to_string().into()))?,
    ))
  }
}

// Error HtsGet response
impl TryFrom<HtsGetError> for FormatJson<Response<ResponseBody>> {
  type Error = http::Error;

  fn try_from(error: HtsGetError) -> http::Result<Self> {
    let (json, status_code) = error.to_json_representation();
    let mut response: Response<ResponseBody> = FormatJson(json).try_into()?;
    *response.status_mut() = status_code;
    Ok(Self(response))
  }
}

/// Handles a response, converting errors to json and using the proper HTTP status code.
fn handle_response(response: Result<JsonResponse>) -> http::Result<worker::Response> {
  match response {
    Err(error) => Ok(FormatJson::try_from(error)?.into_inner()),
    Ok(json) => FormatJson(json).try_into(),
  }
}
