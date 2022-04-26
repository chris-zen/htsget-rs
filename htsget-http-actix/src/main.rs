use std::env::args;

use actix_web::{web, App, HttpServer};
use tokio::select;

use htsget_config::config::{Config, USAGE};
use htsget_http_actix::configure_server;
use htsget_search::storage::local_server::LocalStorageServer;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  if args().len() > 1 {
    // Show help if command line options are provided
    println!("{}", USAGE);
    return Ok(());
  }

  let config = envy::from_env::<Config>().expect("The environment variables weren't properly set!");
  let address = format!("{}:{}", config.htsget_ip, config.htsget_port);
  let local_storage_server = LocalStorageServer::new(
    &config.htsget_localstorage_ip,
    &config.htsget_localstorage_port,
  );
  select! {
    local_server = local_storage_server.start_server("")? => Ok(local_server??),
    actix_server = HttpServer::new(move || {
      App::new().configure(|service_config: &mut web::ServiceConfig| {
        configure_server(service_config, config.clone(), local_storage_server.clone());
      })
    })
    .bind(address)?
    .run() => actix_server
  }
}
