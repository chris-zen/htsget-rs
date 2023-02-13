use std::sync::Arc;

use htsget_http::get_service_info_json as get_base_service_info_json;
use htsget_http::Endpoint;
use htsget_search::htsget::HtsGet;
use worker::ResponseBody;

use crate::handlers::FormatJson;
use crate::ServiceInfo;
use http::Response;

/// Service info endpoint.
pub fn get_service_info_json<H: HtsGet + Send + Sync + 'static>(
  searcher: Arc<H>,
  endpoint: Endpoint,
  config: &ServiceInfo,
) -> http::Result<Response<ResponseBody>> {
  FormatJson(get_base_service_info_json(endpoint, searcher, config)).try_into()
}