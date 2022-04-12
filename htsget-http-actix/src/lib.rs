#[cfg(feature = "async")]
use std::sync::Arc;

use actix_web::web;

use htsget_config::config::HtsgetConfig;
use htsget_config::regex_resolver::RegexResolver;
#[cfg(not(feature = "async"))]
use htsget_search::htsget::blocking::from_storage::HtsGetFromStorage;
#[cfg(not(feature = "async"))]
use htsget_search::htsget::blocking::HtsGet;
#[cfg(feature = "async")]
use htsget_search::htsget::from_storage::HtsGetFromStorage;
#[cfg(feature = "async")]
use htsget_search::htsget::HtsGet;
#[cfg(not(feature = "async"))]
use htsget_search::storage::blocking::local::LocalStorage;
#[cfg(feature = "async")]
use htsget_search::storage::local::LocalStorage;

// Async
#[cfg(feature = "async")]
use crate::handlers::{get, post, reads_service_info, variants_service_info};
// Blocking
#[cfg(not(feature = "async"))]
use crate::handlers::blocking::{get, post, reads_service_info, variants_service_info};

pub mod handlers;

#[cfg(feature = "async")]
pub type AsyncHtsGetStorage = HtsGetFromStorage<LocalStorage>;
#[cfg(not(feature = "async"))]
pub type HtsGetStorage = HtsGetFromStorage<LocalStorage>;

#[cfg(feature = "async")]
pub struct AsyncAppState<H: HtsGet> {
  pub htsget: Arc<H>,
  pub config: HtsgetConfig,
}

#[cfg(not(feature = "async"))]
pub struct AppState<H: HtsGet> {
  pub htsget: H,
  pub config: HtsgetConfig,
}

#[cfg(feature = "async")]
pub fn async_configure_server(service_config: &mut web::ServiceConfig, config: HtsgetConfig) {
  let htsget_path = config.htsget_path.clone();
  let regex_match = config.htsget_regex_match.clone();
  let regex_substitution = config.htsget_regex_substitution.clone();
  service_config
    .app_data(web::Data::new(AsyncAppState {
      htsget: Arc::new(AsyncHtsGetStorage::new(
        LocalStorage::new(
          htsget_path,
          RegexResolver::new(&regex_match, &regex_substitution).unwrap(),
        )
        .expect("Couldn't create a Storage with the provided path"),
      )),
      config,
    }))
    .service(
      web::scope("/reads")
        .route(
          "/service-info",
          web::get().to(reads_service_info::<AsyncHtsGetStorage>),
        )
        .route(
          "/service-info",
          web::post().to(reads_service_info::<AsyncHtsGetStorage>),
        )
        .route("/{id:.+}", web::get().to(get::reads::<AsyncHtsGetStorage>))
        .route(
          "/{id:.+}",
          web::post().to(post::reads::<AsyncHtsGetStorage>),
        ),
    )
    .service(
      web::scope("/variants")
        .route(
          "/service-info",
          web::get().to(variants_service_info::<AsyncHtsGetStorage>),
        )
        .route(
          "/service-info",
          web::post().to(variants_service_info::<AsyncHtsGetStorage>),
        )
        .route(
          "/{id:.+}",
          web::get().to(get::variants::<AsyncHtsGetStorage>),
        )
        .route(
          "/{id:.+}",
          web::post().to(post::variants::<AsyncHtsGetStorage>),
        ),
    );
}

#[cfg(not(feature = "async"))]
pub fn configure_server(service_config: &mut web::ServiceConfig, config: HtsgetConfig) {
  let htsget_path = config.htsget_path.clone();
  let regex_match = config.htsget_regex_match.clone();
  let regex_substitution = config.htsget_regex_substitution.clone();
  service_config
    .app_data(web::Data::new(AppState {
      htsget: HtsGetStorage::new(
        LocalStorage::new(
          htsget_path,
          RegexResolver::new(&regex_match, &regex_substitution).unwrap(),
        )
        .expect("Couldn't create a Storage with the provided path"),
      ),
      config,
    }))
    .service(
      web::scope("/reads")
        .route(
          "/service-info",
          web::get().to(reads_service_info::<HtsGetStorage>),
        )
        .route(
          "/service-info",
          web::post().to(reads_service_info::<HtsGetStorage>),
        )
        .route("/{id:.+}", web::get().to(get::reads::<HtsGetStorage>))
        .route("/{id:.+}", web::post().to(post::reads::<HtsGetStorage>)),
    )
    .service(
      web::scope("/variants")
        .route(
          "/service-info",
          web::get().to(variants_service_info::<HtsGetStorage>),
        )
        .route(
          "/service-info",
          web::post().to(variants_service_info::<HtsGetStorage>),
        )
        .route("/{id:.+}", web::get().to(get::variants::<HtsGetStorage>))
        .route("/{id:.+}", web::post().to(post::variants::<HtsGetStorage>)),
    );
}

#[cfg(test)]
mod tests {
  use actix_web::{App, test, web};
  use actix_web::web::Bytes;
  use async_trait::async_trait;

  use htsget_test_utils::{
    Header as TestHeader, Response as TestResponse, server_tests, TestRequest, TestServer,
  };

  use super::*;
  #[cfg(feature = "async")]
  use super::async_configure_server as configure_server;
  #[cfg(not(feature = "async"))]
  use super::configure_server;

  struct ActixTestServer {
    config: HtsgetConfig,
  }

  struct ActixTestRequest<T>(T);

  impl TestRequest for ActixTestRequest<test::TestRequest> {
    fn insert_header(self, header: TestHeader<impl Into<String>>) -> Self {
      Self(self.0.insert_header(header.into_tuple()))
    }

    fn set_payload(self, payload: impl Into<String>) -> Self {
      Self(self.0.set_payload(payload.into()))
    }

    fn uri(self, uri: impl Into<String>) -> Self {
      Self(self.0.uri(&uri.into()))
    }

    fn method(self, method: impl Into<String>) -> Self {
      Self(
        self
          .0
          .method(method.into().parse().expect("Expected valid method.")),
      )
    }
  }

  impl Default for ActixTestServer {
    fn default() -> Self {
      Self {
        config: server_tests::default_test_config(),
      }
    }
  }

  #[async_trait(?Send)]
  impl TestServer<ActixTestRequest<test::TestRequest>> for ActixTestServer {
    fn get_config(&self) -> &HtsgetConfig {
      &self.config
    }

    fn get_request(&self) -> ActixTestRequest<test::TestRequest> {
      ActixTestRequest(test::TestRequest::default())
    }

    async fn test_server(&self, request: ActixTestRequest<test::TestRequest>) -> TestResponse {
      let app = test::init_service(App::new().configure(
        |service_config: &mut web::ServiceConfig| {
          configure_server(service_config, self.config.clone());
        },
      ))
      .await;
      let response = request.0.send_request(&app).await;
      let status: u16 = response.status().into();
      let bytes: Bytes = test::read_body(response).await;
      TestResponse::new(status, bytes)
    }
  }

  #[actix_web::test]
  async fn test_get() {
    server_tests::test_get(&ActixTestServer::default()).await;
  }

  #[actix_web::test]
  async fn test_post() {
    server_tests::test_post(&ActixTestServer::default()).await;
  }

  #[actix_web::test]
  async fn test_parameterized_get() {
    server_tests::test_parameterized_get(&ActixTestServer::default()).await;
  }

  #[actix_web::test]
  async fn test_parameterized_post() {
    server_tests::test_parameterized_post(&ActixTestServer::default()).await;
  }

  #[actix_web::test]
  async fn test_service_info() {
    server_tests::test_service_info(&ActixTestServer::default()).await;
  }
}
