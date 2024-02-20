pub mod db;
pub mod structures;

use crate::structures::models::User;
use crate::{
    db::{Data, EventMessage},
    structures::{
        rand,
        websocket::{actions::ActionEnum, events::EventEnum},
    },
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        RawQuery, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Extension, Router,
};
use dotenv::dotenv;
use futures_util::stream::SplitStream;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use std::{env, sync::Arc, time::Duration};
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    println!("INIT: env");
    dotenv().ok();
    let db = Data::start(false)
        .await
        .unwrap()
        .inject_content()
        .await
        .unwrap();
    println!("INIT: database");

    println!("INIT: TCP");
    let listener = tokio::net::TcpListener::bind(env::var("BIND").unwrap())
        .await
        .unwrap();
    println!("INIT: bound to {}", listener.local_addr().unwrap());
    axum::serve(listener, app(db)).await.unwrap();
}

fn app(db: Data) -> Router {
    Router::new().route("/", get(main_session_handle).layer(Extension(db)))
}

// -w- upgrade shit
async fn main_session_handle(
    ws: WebSocketUpgrade,
    RawQuery(query): RawQuery,
    Extension(db): Extension<Data>,
) -> Response {
    ws.on_upgrade(|socket| async move {
        let (events, actions) = socket.split();
        let (events, actions) = (
            Arc::new(RwLock::new(events)),
            Arc::new(RwLock::new(actions)),
        );

        let Ok(Some((user, connection))) = db.authenticate(query).await else {
            let _ = events
                .write()
                .await
                .send(Message::Text(
                    "A session token is required for websocket".to_string(),
                ))
                .await;
            println!("early return due to invalid token");
            return;
        };
        tokio::join!(
            events_handler(db.clone(), user.id.clone(), events.clone()),
            action_handler(db, user, events, actions, connection),
        );
    })
}

pub async fn action_handler(
    db: Data,
    user: User,
    events: Arc<RwLock<SplitSink<WebSocket, Message>>>,
    actions: Arc<RwLock<SplitStream<WebSocket>>>,
    connection: String,
) {
    let (channels, users) = db.establish(&user.id).await.unwrap();
    let _ = events
        .write()
        .await
        .send(EventEnum::Establish { channels, users, version: env!("CARGO_PKG_VERSION").to_string() }.into())
        .await;

    while let Some(Ok(msg)) = actions.write().await.next().await {
        let Ok(msg) = msg.to_text() else {
            println!("message body is not text nor bytes");
            continue;
        };
        let Ok(msg) = serde_json::from_str::<ActionEnum>(msg) else {
            println!("unable to deserialize");
            continue;
        };

        match msg {
            ActionEnum::Establish => {}
            ActionEnum::Ping { data } => {
                let _ = events
                    .write()
                    .await
                    .send(EventEnum::Pong { data }.into())
                    .await;
            }
            ActionEnum::MessageSend {
                content,
                reply,
                channel,
            } => {
                // integrity check
                let Ok(Some(mut channel)) = db.get_channel_one(&user.id, &channel).await else {
                    continue;
                };
                if content.clone().replace([' ', '\n'], "").is_empty() {
                    continue;
                };
                channel.members.shift_remove(&user.id);
                db.state.pending_messages.write().await.push(EventMessage {
                    author: user.id.clone(),
                    targets: channel.members,
                    item: EventEnum::MessageSend {
                        id: rand(),
                        author: user.id.clone(),
                        content,
                        reply,
                        channel: channel.id,
                    },
                });
            }
            ActionEnum::MessageEdit { .. } => {}
            ActionEnum::MessageDelete { .. } => {}
            ActionEnum::TypeStatus { typing, channel } => {
                let Ok(Some(mut channel)) = db.get_channel_one(&user.id, &channel).await else {
                    continue;
                };
                channel.members.shift_remove(&user.id);
                db.state.pending_messages.write().await.push(EventMessage {
                    author: user.id.clone(),
                    targets: channel.members,
                    item: EventEnum::TypeStatus {
                        typing,
                        channel: channel.id,
                        user: user.id.clone(),
                    },
                });
            }
        }
    }
    // assumed disconnected
    let _ = db.logout(user.id, connection).await;
}
pub async fn events_handler(
    db: Data,
    user_id: String,
    events: Arc<RwLock<SplitSink<WebSocket, Message>>>,
) {
    loop {
        tokio::time::sleep(Duration::from_millis(1)).await;
        let messages = db.state.pending_messages.read().await.clone();
        for (index, message) in messages.into_iter().enumerate() {
            if message.clone().targets.contains(&user_id) {
                let _ = events.write().await.send(message.clone().item.into()).await;

                let mut message = message.clone();
                message.targets.shift_remove(&user_id);
                if !message.targets.is_empty() {
                    db.state.pending_messages.write().await.remove(index);
                } else {
                    // WARNING: UNSAFE CALL
                    db.state.pending_messages.write().await[index].targets = message.targets;
                }
            }
        }
    }
}
