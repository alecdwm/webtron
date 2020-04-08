use bytes::{Buf, Bytes};
use futures::sink::SinkExt;
use futures::stream::{Stream, StreamExt};
use hyper::header::{Entry, HeaderValue, CONNECTION, HOST, UPGRADE};
use hyper::{Body, Client, HeaderMap, Method, Request, StatusCode, Uri};
use log::{error, warn};
use std::error::Error as StdError;
use std::net::SocketAddr;
use tokio::select;
use tokio_tungstenite::tungstenite::protocol::{Message, Role};
use tokio_tungstenite::WebSocketStream;
use warp::path::Tail;
use warp::ws::{Message as WarpMessage, Ws};
use warp::{Filter, Rejection, Reply};

use crate::web::errors::{BadGateway, InternalServerError};

const X_FORWARDED_FOR: &str = "x-forwarded-for";
const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

pub fn proxy(target: &str) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let target: Uri = target.parse().expect("Invalid proxy target");
    let target_host = match (target.clone().host(), target.clone().port()) {
        (Some(host), Some(port)) => HeaderValue::from_str(&format!("{}:{}", host, port)).ok(),
        (Some(host), None) => HeaderValue::from_str(host).ok(),
        _ => None,
    }
    .expect("Failed to parse target host");

    let target = warp::any().map(move || (target.clone(), target_host.clone()));
    let empty_query = warp::any().map(move || "".to_owned());

    let ws = warp::ws()
        .and(warp::path::tail())
        .and(warp::query::raw().or(empty_query).unify())
        .and(warp::header::headers_cloned())
        .and(warp::addr::remote())
        .and(target.clone())
        .and_then(proxy_ws);

    let http = warp::path::tail()
        .and(warp::query::raw().or(empty_query).unify())
        .and(warp::body::stream())
        .and(warp::header::headers_cloned())
        .and(warp::method())
        .and(warp::addr::remote())
        .and(target)
        .and_then(proxy_http);

    ws.or(http)
}

async fn proxy_ws(
    ws: Ws,
    path: Tail,
    query: String,
    headers: HeaderMap,
    remote_addr: Option<SocketAddr>,
    (target, target_host): (Uri, HeaderValue),
) -> Result<impl Reply, Rejection> {
    let client = Client::new();

    let mut request = Request::builder()
        .uri(format_uri(&target, path.as_str(), query))
        .body(Body::empty())
        .map_err(|error| {
            error!("Failed to construct websocket proxy request: {}", error);
            warp::reject::custom(InternalServerError)
        })?;

    *request.headers_mut() = headers;
    remove_hop_by_hop_headers(request.headers_mut());
    request.headers_mut().insert(HOST, target_host);
    append_x_forwarded_for_header(remote_addr, request.headers_mut());
    insert_websocket_upgrade_headers(request.headers_mut());

    let upstream_response = client.request(request).await.map_err(|error| {
        warn!("Error occurred while proxying websocket request: {}", error);
        warp::reject::custom(BadGateway)
    })?;

    if upstream_response.status() != StatusCode::SWITCHING_PROTOCOLS {
        warn!("Upstream server didn't switch protocols");
        return Err(warp::reject::custom(BadGateway));
    }

    let upstream_websocket = match upstream_response.into_body().on_upgrade().await {
        Ok(upgraded) => WebSocketStream::from_raw_socket(upgraded, Role::Client, None).await,
        Err(error) => {
            warn!("Failed to upgrade upstream connection: {}", error);
            return Err(warp::reject::custom(BadGateway));
        }
    };

    Ok(ws.on_upgrade(|websocket| async move {
        let (mut tx, mut rx) = websocket.split();
        let (mut upstream_tx, mut upstream_rx) = upstream_websocket.split();

        let in_task = tokio::spawn(async move {
            while let Some(message) = rx.next().await {
                let message = match message {
                    Ok(message) => message,
                    Err(error) => {
                        error!("Error occurred in incoming message: {}", error);
                        break;
                    }
                };

                let message = match message.to_str() {
                    Ok(message) => Message::text(message),
                    Err(()) => {
                        if message.is_binary() {
                            Message::binary(message)
                        } else if message.is_ping() {
                            Message::Ping(message.into_bytes())
                        } else if message.is_pong() {
                            Message::Pong(message.into_bytes())
                        } else if message.is_close() {
                            Message::Close(None)
                        } else {
                            warn!("Not proxying websocket message {:?}", message);
                            continue;
                        }
                    }
                };

                upstream_tx.send(message).await.unwrap_or_else(|error| {
                    error!("Failed to proxy incoming websocket message: {}", error)
                });
            }
        });
        let out_task = tokio::spawn(async move {
            while let Some(message) = upstream_rx.next().await {
                let message = match message {
                    Ok(message) => message,
                    Err(error) => {
                        error!("Error occurred in outgoing message: {}", error);
                        break;
                    }
                };

                let message = match message {
                    Message::Text(text) => WarpMessage::text(text),
                    Message::Binary(data) => WarpMessage::binary(data),
                    Message::Ping(data) => WarpMessage::ping(data),
                    Message::Pong(_) => {
                        warn!("Not proxying websocket message {:?}", message);
                        continue;
                    }
                    Message::Close(_) => WarpMessage::close(),
                };

                tx.send(message).await.unwrap_or_else(|error| {
                    error!("Failed to proxy outgoing websocket message: {}", error)
                });
            }
        });

        if let Err(error) = select! {
            out = in_task => out,
            out = out_task => out,
        } {
            error!("Failure occurred while handling websocket: {}", error);
        }
    }))
}

async fn proxy_http(
    path: Tail,
    query: String,
    body: impl Stream<Item = Result<impl Buf, warp::Error>> + Unpin + Send + Sync + 'static,
    headers: HeaderMap,
    method: Method,
    remote_addr: Option<SocketAddr>,
    (target, target_host): (Uri, HeaderValue),
) -> Result<impl Reply, Rejection> {
    let client = Client::new();

    let body: Box<dyn Stream<Item = Result<Bytes, Box<dyn StdError + Send + Sync>>> + Send + Sync> =
        Box::new(body.map(|result| {
            result.map(|mut buf| buf.to_bytes()).map_err(|error| {
                error!("Error occurred while reading request body: {}", error);
                error.into()
            })
        }));

    let mut request = Request::builder()
        .method(method)
        .uri(format_uri(&target, path.as_str(), query))
        .body(Body::from(body))
        .map_err(|error| {
            error!("Failed to construct proxy request: {}", error);
            warp::reject::custom(InternalServerError)
        })?;

    *request.headers_mut() = headers;
    remove_hop_by_hop_headers(request.headers_mut());
    request.headers_mut().insert(HOST, target_host);
    append_x_forwarded_for_header(remote_addr, request.headers_mut());

    let mut response = client.request(request).await.map_err(|error| {
        warn!("Error occurred while proxying request: {}", error);
        warp::reject::custom(BadGateway)
    })?;

    remove_hop_by_hop_headers(response.headers_mut());

    Ok(response)
}

fn format_uri(target: &Uri, path: &str, query: String) -> String {
    let query = if !query.is_empty() {
        format!("?{}", query)
    } else {
        query
    };
    format!("{}{}{}", target, path, query)
}
fn remove_hop_by_hop_headers(headers: &mut HeaderMap) {
    for header in HOP_BY_HOP_HEADERS {
        headers.remove(*header);
    }
}
fn append_x_forwarded_for_header(remote_addr: Option<SocketAddr>, headers: &mut HeaderMap) {
    if let Some(remote_addr) = remote_addr {
        if let Ok(remote_addr) = HeaderValue::from_str(&remote_addr.ip().to_string()) {
            match headers.entry(X_FORWARDED_FOR) {
                Entry::Vacant(entry) => {
                    entry.insert(remote_addr);
                }
                Entry::Occupied(mut entry) => {
                    let existing = entry.get_mut();
                    if let Ok(updated) = HeaderValue::from_bytes(
                        &[existing.as_bytes(), b", ", remote_addr.as_bytes()].concat(),
                    ) {
                        *existing = updated;
                    } else {
                        *existing = remote_addr;
                    }
                }
            }
        }
    };
}
fn insert_websocket_upgrade_headers(headers: &mut HeaderMap) {
    headers.insert(CONNECTION, HeaderValue::from_static("upgrade"));
    headers.insert(UPGRADE, HeaderValue::from_static("websocket"));
}
