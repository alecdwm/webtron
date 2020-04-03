mod websocket_client;

use actix::Addr;
use actix_files::Files;
use actix_web::dev::Server as ActixServer;
use actix_web::{web, App, HttpServer};
use anyhow::{anyhow, Context, Error};

use crate::config::Config;
use crate::server::Server as WebtronServer;
use websocket_client::websocket_route;

pub fn start(server_address: Addr<WebtronServer>, config: &Config) -> Result<ActixServer, Error> {
    Ok(HttpServer::new(move || {
        App::new()
            .data(server_address.clone())
            //
            // websocket handler
            //
            .service(web::resource("/ws").to(websocket_route))
            //
            // fs handler
            //
            .service(Files::new("/", "client/public").index_file("index.html"))
    })
    .bind(config.bind_address)
    .with_context(|| anyhow!("Failed to bind to socket {}", config.bind_address))?
    .run())
}
