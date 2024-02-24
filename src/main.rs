pub mod db;
pub mod structures;

use crate::{
    db::{Data, EventMessage},
    structures::models::User,
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
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use dotenv::dotenv;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use std::{env, sync::Arc, time::Duration};
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    println!("INIT: env");
    dotenv().ok();
    let db = Data::start(false)
        .await
        .expect("failed to load database")
        .inject_content()
        .await
        .expect("failed to initialise fake data");
    println!("INIT: database");

    println!("INIT: TCP");
    let listener = tokio::net::TcpListener::bind(env::var("BIND").unwrap())
        .await
        .expect("failed to bind to local port");
    println!("INIT: bound to {}", listener.local_addr().unwrap());
    axum::serve(listener, app(db)).await.unwrap();
}

fn app(db: Data) -> Router {
    Router::new()
        .route("/ws", get(main_session_handle).layer(Extension(db)))
        .route("/available", get(handler_running))
}

async fn handler_running() -> impl IntoResponse {
    (StatusCode::NO_CONTENT, "")
}

/// provides a handler for the upgraded websocket session, once session authentication is properly handled
/// it starts the 'events' and 'actions' handler to manage incoming and outgoing connections
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
/// # action handler
/// this function reads incoming requests, depending on applicability and actions it adds events
/// to the memory database, this function is also responsible for sending establish
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
        .send(
            EventEnum::Establish {
                channels,
                users,
                version: env!("CARGO_PKG_VERSION").to_string(),
            }
            .into(),
        )
        .await;

    while let Some(Ok(msg)) = actions.write().await.next().await {
        let Ok(msg) = msg.to_text() else {
            println!(
                "message body is not text nor bytes, sent by {} ({})",
                &user.username, &user.id
            );
            continue;
        };
        let Ok(msg) = serde_json::from_str::<ActionEnum>(msg) else {
            println!(
                "unable to deserialize, sent by {} ({})",
                &user.username, &user.id
            );
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

            ActionEnum::TypeStatus { typing, channel } => {
                let Ok(Some(mut channel)) = db.get_channel_one(&user.id, &channel).await else {
                    continue;
                };
                channel.members.shift_remove(&user.id);
                db.state.pending_messages.write().await.push(EventMessage {
                    author: user.id.clone(),
                    targets: channel.members,
                    item: EventEnum::TypeStatus {
                        typing: typing.unwrap_or_default(),
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

/// # event_handler
/// this function reads from the memory database for pending events, and after a certain time interval
/// it sends all pending events to the specified user
/// the performance for this model at scale is unknown, and may be migrated to redis or mongo
pub async fn events_handler(
    db: Data,
    user_id: String,
    events: Arc<RwLock<SplitSink<WebSocket, Message>>>,
) {
    let delay = env::var("EVENT_WAIT")
        .unwrap_or_default()
        .parse::<u64>()
        .unwrap_or(1);
    loop {
        tokio::time::sleep(Duration::from_millis(delay)).await;
        let messages = db.state.pending_messages.read().await.clone();
        for (index, message) in messages.into_iter().enumerate() {
            if message.clone().targets.contains(&user_id) {
                let _ = events.write().await.send(message.clone().item.into()).await;

                let mut message = message.clone();
                message.targets.shift_remove(&user_id);
                if !message.targets.is_empty() {
                    db.state.pending_messages.write().await.remove(index);
                } else {
                    // WARNING: UNSAFE CALL todo
                    // this may result in panics, safety status is unknown
                    db.state.pending_messages.write().await[index].targets = message.targets;
                }
            }
        }
    }
}
