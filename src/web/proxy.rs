use bytes::{Buf, Bytes};
use futures::sink::SinkExt;
use futures::stream::{Stream, StreamExt};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, StreamBody};
use hyper::body::Frame;
use hyper::header::{CONNECTION, Entry, HOST, HeaderName, HeaderValue, UPGRADE};
use hyper::{HeaderMap, Method, Request, StatusCode, Uri};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use log::{error, warn};
use std::error::Error as StdError;
use std::net::SocketAddr;
use tokio::select;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol::{Message, Role};
use warp::path::Tail;
use warp::ws::{Message as WarpMessage, Ws};
use warp::{Filter, Rejection, Reply};

use crate::web::errors::{BadGateway, InternalServerError};

type ProxyBody = BoxBody<Bytes, Box<dyn StdError + Send + Sync>>;
type ProxyClient = Client<HttpConnector, ProxyBody>;

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

pub fn proxy(
    target: &str,
) -> impl Filter<Extract = (impl Reply + use<>,), Error = Rejection> + Clone + use<> {
    let target: Uri = target.parse().expect("Invalid proxy target");
    let target_host = match (target.host(), target.port()) {
        (Some(host), Some(port)) => HeaderValue::from_str(&format!("{}:{}", host, port)).ok(),
        (Some(host), None) => HeaderValue::from_str(host).ok(),
        _ => None,
    }
    .expect("Failed to parse target host");

    let client = Client::builder(TokioExecutor::new()).build_http();
    let target = warp::any().map(move || (client.clone(), target.clone(), target_host.clone()));
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
    (client, target, target_host): (ProxyClient, Uri, HeaderValue),
) -> Result<impl Reply, Rejection> {
    let mut request = Request::builder()
        .uri(format_uri(&target, path.as_str(), query))
        .body(Empty::new().map_err(|never| match never {}).boxed())
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

    let upstream_websocket = match hyper::upgrade::on(upstream_response).await {
        Ok(upgraded) => {
            WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Client, None).await
        }
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
                            Message::binary(message.into_bytes())
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

                if let Err(error) = upstream_tx.send(message).await {
                    error!("Failed to proxy incoming websocket message: {}", error);
                    break;
                }
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
                    Message::Text(text) => WarpMessage::text(text.as_str()),
                    Message::Binary(data) => WarpMessage::binary(data),
                    Message::Ping(data) => WarpMessage::ping(data),
                    Message::Pong(_) => {
                        warn!("Not proxying websocket message {:?}", message);
                        continue;
                    }
                    Message::Close(_) => WarpMessage::close(),
                    Message::Frame(_) => continue,
                };

                if let Err(error) = tx.send(message).await {
                    error!("Failed to proxy outgoing websocket message: {}", error);
                    break;
                }
            }
        });

        let (mut in_task, mut out_task) = (in_task, out_task);
        if let Err(error) = select! {
            out = &mut in_task => out,
            out = &mut out_task => out,
        } {
            error!("Failure occurred while handling websocket: {}", error);
        }
        // Once either side has closed, stop forwarding the other.
        in_task.abort();
        out_task.abort();
    }))
}

async fn proxy_http(
    path: Tail,
    query: String,
    body: impl Stream<Item = Result<impl Buf, warp::Error>> + Unpin + Send + Sync + 'static,
    headers: HeaderMap,
    method: Method,
    remote_addr: Option<SocketAddr>,
    (client, target, target_host): (ProxyClient, Uri, HeaderValue),
) -> Result<impl Reply, Rejection> {
    let body = StreamBody::new(body.map(|result| {
        result
            .map(|mut buf| Frame::data(buf.copy_to_bytes(buf.remaining())))
            .map_err(|error| {
                error!("Error occurred while reading request body: {}", error);
                error.into()
            })
    }));

    let mut request = Request::builder()
        .method(method)
        .uri(format_uri(&target, path.as_str(), query))
        .body(BodyExt::boxed(body))
        .map_err(|error| {
            error!("Failed to construct proxy request: {}", error);
            warp::reject::custom(InternalServerError)
        })?;

    *request.headers_mut() = headers;
    remove_hop_by_hop_headers(request.headers_mut());
    request.headers_mut().insert(HOST, target_host);
    append_x_forwarded_for_header(remote_addr, request.headers_mut());

    let response = client.request(request).await.map_err(|error| {
        warn!("Error occurred while proxying request: {}", error);
        warp::reject::custom(BadGateway)
    })?;

    let (mut parts, body) = response.into_parts();
    remove_hop_by_hop_headers(&mut parts.headers);

    let mut reply = warp::reply::stream(body.into_data_stream()).into_response();
    *reply.status_mut() = parts.status;
    *reply.headers_mut() = parts.headers;
    Ok(reply)
}

fn format_uri(target: &Uri, path: &str, query: String) -> String {
    let query = if !query.is_empty() {
        format!("?{}", query)
    } else {
        query
    };
    format!("{}{}{}", target, path, query)
}
/// Removes the headers that only apply to one connection: the standard ones,
/// and any others named in the `Connection` header.
fn remove_hop_by_hop_headers(headers: &mut HeaderMap) {
    let named: Vec<HeaderName> = headers
        .get_all(CONNECTION)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|name| HeaderName::from_bytes(name.trim().as_bytes()).ok())
        .collect();
    for header in named {
        headers.remove(header);
    }
    for header in HOP_BY_HOP_HEADERS {
        headers.remove(*header);
    }
}
fn append_x_forwarded_for_header(remote_addr: Option<SocketAddr>, headers: &mut HeaderMap) {
    if let Some(remote_addr) = remote_addr
        && let Ok(remote_addr) = HeaderValue::from_str(&remote_addr.ip().to_string())
    {
        match headers.entry(X_FORWARDED_FOR) {
            Entry::Vacant(entry) => {
                entry.insert(remote_addr);
            }
            Entry::Occupied(mut entry) => {
                let existing = entry.get_mut();
                match HeaderValue::from_bytes(
                    &[existing.as_bytes(), b", ", remote_addr.as_bytes()].concat(),
                ) {
                    Ok(updated) => {
                        *existing = updated;
                    }
                    _ => {
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
