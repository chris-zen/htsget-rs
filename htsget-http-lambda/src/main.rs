use std::io::ErrorKind::Other;
use std::sync::Arc;

use lambda_http::{service_fn, Error, Request};
use tokio::select;

use htsget_config::config::{Config, StorageType};
use htsget_config::regex_resolver::RegexResolver;
use htsget_http_lambda::Router;
use htsget_search::htsget::from_storage::HtsGetFromStorage;
use htsget_search::storage::aws::AwsS3Storage;
use htsget_search::storage::axum_server::HttpsFormatter;
use htsget_search::storage::local::LocalStorage;

#[tokio::main]
async fn main() -> Result<(), Error> {
  let config = Config::from_env()?;

  match config.htsget_storage_type {
    StorageType::LocalStorage => local_storage_server(config).await,
    #[cfg(feature = "s3-storage")]
    StorageType::AwsS3Storage => s3_storage_server(config).await
  }
}

async fn local_storage_server(config: Config) -> Result<(), Error> {
  let searcher = Arc::new(HtsGetFromStorage::new(
    LocalStorage::new(
      config.htsget_path,
      config.htsget_resolver,
      HttpsFormatter::from(config.htsget_addr)
    )
      .unwrap(),
  ));
  let router = &Router::new(searcher, &config.htsget_config_service_info);

  let handler = |event: Request| async move { Ok(router.route_request(event).await) };
  lambda_http::run(service_fn(handler)).await?;

  Ok(())
}

#[cfg(feature = "s3-storage")]
async fn s3_storage_server(config: Config) -> Result<(), Error> {
  let searcher = Arc::new(HtsGetFromStorage::<AwsS3Storage>::from(config.htsget_s3_bucket, config.htsget_resolver).await?);
  let router = &Router::new(searcher, &config.htsget_config_service_info);

  let handler = |event: Request| async move { Ok(router.route_request(event).await) };
  lambda_http::run(service_fn(handler)).await?;

  Ok(())
}
