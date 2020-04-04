mod proxy;
mod websocket_client;

use actix::Addr;
use actix_web::dev::Server as ActixServer;
use actix_web::{web, App, HttpServer};
use actix_web_static_files::ResourceFiles;
use anyhow::{anyhow, Context, Error};
use std::collections::HashMap;
use url::Url;

use crate::config::Config;
use crate::server::Server as WebtronServer;
use proxy::forward;
use websocket_client::websocket_route;

#[cfg(debug_assertions)]
const DEBUG: bool = true;
#[cfg(not(debug_assertions))]
const DEBUG: bool = false;

#[cfg(debug_assertions)]
fn generate() -> HashMap<&'static str, actix_web_static_files::Resource> {
    HashMap::new()
}
#[cfg(not(debug_assertions))]
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

pub fn start(server_address: Addr<WebtronServer>, config: &Config) -> Result<ActixServer, Error> {
    Ok(HttpServer::new(move || {
        let mut app = App::new()
            .data(server_address.clone())
            .data(Url::parse("http://localhost:3001").expect("Failed to parse url"))
            //
            // websocket handler
            //
            .service(web::resource("/ws").to(websocket_route));

        if DEBUG {
            //
            // proxy handler
            //
            app = app.default_service(web::route().to(forward));
        } else {
            //
            // fs handler
            //
            app = app.service(ResourceFiles::new("/", generate()))
        }
        app
    })
    .bind(config.bind_address)
    .with_context(|| anyhow!("Failed to bind to socket {}", config.bind_address))?
    .run())
}
