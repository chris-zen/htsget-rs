use std::sync::Arc;
use worker::Error;

async fn main() -> Result<(), Error> {
  if let Some(config) = Config::parse_args() {
    let config = Config::from_config(config)?;

    let service_info = config.ticket_server().service_info().clone();
    let cors = config.ticket_server().cors().clone();
    let router = &Router::new(Arc::new(config.owned_resolvers()), &service_info);

    handle_request(cors, router).await
  } else {
    Ok(())
  }
}
