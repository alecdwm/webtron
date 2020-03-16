mod websocket_client;

use actix::Addr;
use actix_files::Files;
use actix_web::dev::Server as ActixServer;
use actix_web::{web, App, HttpRequest, HttpServer};
use actix_web_actors::ws as websocket;
use anyhow::{Context, Error};

use crate::config::Config;
use crate::server::Server as WebtronServer;
use websocket_client::WebsocketClient;

pub fn start(server_address: Addr<WebtronServer>, config: &Config) -> Result<ActixServer, Error> {
    Ok(HttpServer::new(move || {
        let server_address = server_address.clone();

        App::new()
            //
            // websocket handler
            //
            .service(web::resource("/ws").route(web::get().to(
                move |request: HttpRequest, stream: web::Payload| {
                    let server_address = server_address.clone();

                    async move {
                        let ip_address = request
                            .connection_info()
                            .remote()
                            .map(|ip_address| ip_address.to_owned());

                        websocket::start(
                            WebsocketClient::new(ip_address, server_address.clone()),
                            &request,
                            stream,
                        )
                    }
                },
            )))
            //
            // fs handler
            //
            .service(Files::new("/", "client").index_file("index.html"))
    })
    .bind(config.bind_address)
    .with_context(|| format!("Failed to bind to socket {}", config.bind_address))?
    .run())
}
