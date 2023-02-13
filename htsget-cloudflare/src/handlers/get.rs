use std::collections::HashMap;
use std::sync::Arc;

use http::{self, Response};
use worker::ResponseBody;

use htsget_http::{get as htsget_get, Endpoint};
use htsget_search::htsget::HtsGet;

use crate::handlers::handle_response;

/// Get request reads endpoint
pub async fn get<H: HtsGet + Send + Sync + 'static>(
  id_path: String,
  searcher: Arc<H>,
  mut query: HashMap<String, String>,
  endpoint: Endpoint,
) -> http::Result<Response<ResponseBody>> {
  //log_request(query);
  query.insert("id".to_string(), id_path);
  handle_response(htsget_get(searcher, query, endpoint).await)
}
