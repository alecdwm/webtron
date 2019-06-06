mod websocket_client;

use self::websocket_client::WebsocketClient;
use crate::config::Config;
use crate::server::Server as WebtronServer;
use actix::Addr;
use actix_files::Files;
use actix_server::Server as ActixServer;
use actix_web::{web, App, HttpServer};
use actix_web_actors::ws as websocket;
use failure::{Error, ResultExt};

pub fn start(server_addr: Addr<WebtronServer>, config: &Config) -> Result<ActixServer, Error> {
    Ok(HttpServer::new(move || {
        let server_addr = server_addr.clone();

        App::new()
            //
            // websocket handler
            //
            .service(web::resource("/ws").route(web::get().to(
                move |request, stream: web::Payload| {
                    websocket::start(WebsocketClient::new(server_addr.clone()), &request, stream)
                },
            )))
            //
            // fs handler
            //
            .service(Files::new("/", "client").index_file("index.html"))
    })
    .bind(config.bind_addr)
    .with_context(|_| format!("Failed to bind to socket {}", config.bind_addr))?
    .start())
}
