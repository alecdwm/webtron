use crate::config::Config;
use crate::server::{ClientConnection, ServerMessage};
// use failure::Error;
// use failure::ResultExt;
use futures::{Future, Sink, Stream};
use std::sync::{mpsc, Arc, Mutex};
use uuid::Uuid;
use warp::{filters::BoxedFilter, Filter, Reply};

// implementation help:
// https://github.com/seanmonstar/warp/blob/master/examples/websockets_chat.rs
// https://github.com/seanmonstar/warp/blob/master/examples/todos.rs

pub fn run(server_tx: mpsc::Sender<ServerMessage>, config: &Config) {
    // TODO: Consider using channels to send network events
    // to single-threaded WebtronServer instead of locking
    // access to WebtronServer from multiple threads
    //
    // See: https://doc.rust-lang.org/book/ch16-02-message-passing.html

    let routes = get_ws(server_tx).or(get_root());
    warp::serve(routes).run(config.bind_addr);
}

/// GET /
fn get_root() -> BoxedFilter<(impl Reply,)> {
    warp::get2().and(warp::filters::fs::dir("client")).boxed()
}

/// GET /ws
fn get_ws(server_tx: mpsc::Sender<ServerMessage>) -> BoxedFilter<(impl Reply,)> {
    let server_tx = Arc::new(Mutex::new(server_tx));

    warp::get2()
        .and(warp::path("ws"))
        .and(warp::ws::ws2())
        .map(move |ws: warp::ws::Ws2| {
            let server_tx = server_tx
                .lock()
                .expect("Failed to lock server_tx for new client")
                .clone();

            ws.on_upgrade(move |websocket| {
                let client_id = Uuid::new_v4();
                let (c2s_tx, c2s_rx) = mpsc::channel();
                // let (s2c_tx, s2c_rx) = mpsc::channel();

                let (websocket_tx, websocket_rx) = websocket.split();

                // server_tx
                //     .send(ServerMessage::ClientConnected(ClientConnection {
                //         id: client_id,
                //         tx: s2c_tx,
                //         rx: c2s_rx,
                //     }))
                //     .expect("Failed to send new client connection to webtron server");

                // websocket_rx.for_each(move |message| {
                //     dbg!(&message);
                // })
                // s2c_rx
                //     .forward(websocket_tx)
                //     .join(websocket_rx.forward(c2s_tx));

                // warp::spawn(
                //     s2c_rx
                //         .map_err(|()| -> warp::Error { unreachable!("unbounded rx never errors") })
                //         .for_each(move |outgoing_message| {
                //             let json = serde_json::to_string(&outgoing_message)
                //                 .expect("Unable to serialize outgoing_message");
                //             websocket_tx.send(warp::ws::Message::text(json));
                //             Ok(())
                //         })
                //         .map_err(move |error| {
                //             // TODO: handle error
                //             eprintln!("websocket send error: {:?}", error);
                //         }),
                // );

                websocket_rx
                    .for_each(move |message| {
                        let incoming_message = match &message.to_str() {
                            Ok(val) => match serde_json::from_str(val) {
                                Ok(val) => val,
                                Err(error) => {
                                    // TODO: Log 'error parsing incoming message'
                                    //        or 'ignoring incorrectly formatted incoming message'
                                    eprintln!("Error paring incoming message: {:?}", error);
                                    // return Err(error);
                                    return Ok(());
                                }
                            },
                            Err(_) => {
                                // TODO: Log 'ignoring incoming non-text message'
                                return Ok(());
                            }
                        };
                        c2s_tx.send(incoming_message);
                        // .expect("Failed to send incoming message to webtron server");

                        Ok(())
                    })
                    .then(move |result| {
                        server_tx.send(ServerMessage::ClientDisconnected(client_id));
                        // .expect("Failed to send client disconnect to webtron server");
                        // TODO: handle disconnect
                        result
                    })
                    .map_err(move |error| {
                        // TODO: handle error
                        eprintln!("websocket error: {:?}", error);
                    })
                // .join(|| {
                //     s2c_rx.try_iter();
                //     Ok(()).into_future()
                // })
                // tx.send("Hello!");

                // server_tx.send(WebtronServerMessage::NewClient(websocket));

                // Ok(()).into_future()
                // rx.forward(tx).map(|_| ()).map_err(|e| {
                //     eprintln!("websocket error: {:?}", e);
                // })
            })
        })
        .boxed()
}
