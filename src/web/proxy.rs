use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_codec::Framed;
use actix_web::client::Client;
use actix_web::error::InternalError;
use actix_web::http::StatusCode;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use actix_web_actors::ws::WebsocketContext;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use log::{error, warn};
use std::time::Duration;
use url::Url;

pub async fn forward(
    req: HttpRequest,
    mut body: web::Payload,
    url: web::Data<Url>,
) -> Result<HttpResponse, Error> {
    let mut url = url.as_ref().clone();
    url.set_path(req.uri().path());
    url.set_query(req.uri().query());

    let host = match (
        url.host_str().expect("Proxy target missing host"),
        url.port(),
    ) {
        (host, Some(port)) => format!("{}:{}", host, port),
        (host, None) => host.to_owned(),
    };

    if req
        .headers()
        .get_all("Upgrade")
        .all(|upgrade| upgrade != "websocket")
    {
        let forwarded_req = Client::new()
            .request_from(url.as_str(), req.head())
            .set_header("host", host)
            .timeout(Duration::from_secs(60));
        let forwarded_req = if let Some(addr) = req.head().peer_addr {
            forwarded_req.header("x-forwarded-for", format!("{}", addr.ip()))
        } else {
            forwarded_req
        };

        let mut body_bytes = web::BytesMut::new();
        while let Some(item) = body.next().await {
            body_bytes.extend_from_slice(&item?);
        }

        let reversed_res = forwarded_req.send_body(body_bytes).await.map_err(|error| {
            error!("Proxy error: {}", error);
            Error::from(error)
        })?;

        let mut res = HttpResponse::build(reversed_res.status());
        for (header_name, header_value) in reversed_res
            .headers()
            .iter()
            .filter(|(h, _)| *h != "connection")
        {
            res.header(header_name.clone(), header_value.clone());
        }

        Ok(res.streaming(reversed_res))
    } else {
        let forwarded_ws = Client::new().ws(url.as_str());
        let forwarded_ws = req
            .headers()
            .iter()
            .fold(forwarded_ws, |forwarded_ws, (header_name, header_value)| {
                forwarded_ws.set_header(header_name.to_owned(), header_value.to_owned())
            })
            .set_header("host", host);
        let forwarded_ws = if let Some(addr) = req.head().peer_addr {
            forwarded_ws.header("x-forwarded-for", format!("{}", addr.ip()))
        } else {
            forwarded_ws
        };

        let server_framed = forwarded_ws
            .connect()
            .await
            .map(|(_, framed)| framed)
            .map_err(|error| {
                error!("WS proxy error: {}", error);
                InternalError::new(error, StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        let mut res = ws::handshake(&req).map_err(|error| {
            error!("WS proxy error: {}", error);
            Error::from(error)
        })?;

        let body_stream = body;

        Ok(res.streaming(WebsocketContext::create(
            WebsocketProxy::from(server_framed),
            body_stream,
        )))
    }
}

struct WebsocketProxy {
    server_sink: Option<SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    server_sink_write: Option<SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>>,
    server_stream: Option<SplitStream<Framed<BoxedSocket, Codec>>>,
}

impl WebsocketProxy {
    fn from(server_framed: Framed<BoxedSocket, Codec>) -> Self {
        let (server_sink, server_stream) = server_framed.split();

        Self {
            server_sink: Some(server_sink),
            server_sink_write: None,
            server_stream: Some(server_stream),
        }
    }
}

impl Actor for WebsocketProxy {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        self.server_sink_write = Some(SinkWrite::new(
            self.server_sink
                .take()
                .expect("WebsocketProxy server_sink required"),
            context,
        ));

        context.add_stream(
            self.server_stream
                .take()
                .expect("WebsocketProxy server_stream required"),
        );
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketProxy {
    fn handle(
        &mut self,
        message: Result<ws::Message, ws::ProtocolError>,
        _context: &mut Self::Context,
    ) {
        let message = unwrap_or_return!(message, |error| error!("WS proxy error: {}", error));

        self.server_sink_write
            .as_mut()
            .expect("WebsocketProxy server_sink_write required")
            .write(message)
            .unwrap_or_else(|error| error!("WS proxy error: {}", error));
    }
}

impl StreamHandler<Result<Frame, ws::ProtocolError>> for WebsocketProxy {
    fn handle(&mut self, message: Result<Frame, ws::ProtocolError>, context: &mut Self::Context) {
        let message = unwrap_or_return!(message, |error| error!("WS proxy error: {}", error));

        match message {
            Frame::Text(text) => {
                context.text(unwrap_or_return!(
                    String::from_utf8(text.to_vec()),
                    |error| error!("WS proxy error: {}", error)
                ));
            }
            Frame::Binary(binary) => {
                context.binary(binary);
            }
            Frame::Ping(message) => {
                context.ping(&message);
            }
            Frame::Pong(message) => {
                context.pong(&message);
            }
            Frame::Close(reason) => {
                context.close(reason);
            }
            Frame::Continuation(message) => warn!(
                "Ignoring websocket continuation frame from server: {:?}",
                message
            ),
        }
    }

    fn finished(&mut self, context: &mut Self::Context) {
        context.stop()
    }
}

impl WriteHandler<ws::ProtocolError> for WebsocketProxy {}
