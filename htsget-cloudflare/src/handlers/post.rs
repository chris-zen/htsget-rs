use std::sync::Arc;

use htsget_http::{post as htsget_post, Endpoint, PostRequest};
use htsget_search::htsget::HtsGet;
use worker::ResponseBody;

use crate::handlers::handle_response;
use http::Response;

/// Post request reads endpoint
pub async fn post<H: HtsGet + Send + Sync + 'static>(
  id_path: String,
  searcher: Arc<H>,
  query: PostRequest,
  endpoint: Endpoint,
) -> http::Result<Response<ResponseBody>> {
  handle_response(htsget_post(searcher, query, id_path, endpoint).await)
}