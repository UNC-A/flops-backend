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
        .route("/ws/", get(main_session_handle).layer(Extension(db)))
        .route("/available/", get(handler_running))
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

        let Ok(Some(user)) = db.authenticate(query).await else {
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
        if !db.state.user_online(&user.id).await {
            let _ = events
                .write()
                .await
                .send(Message::Text(format!(
                    "{} ({}) is already online, simultaneous connections are not allowed",
                    &user.username, &user.id
                )))
                .await;
            return;
        }
        tokio::join!(
            events_handler(db.clone(), user.id.clone(), events.clone()),
            action_handler(db.clone(), user.clone(), events, actions),
        );
    })
}
// todo very very bugged
// async fn action_egg(db: Data) {
//     let author = "dsfgdsufygsduygds".to_string();
//     loop {
//         tokio::time::sleep(Duration::from_secs(5)).await;
//         println!(
//             "\n\n\n{:?}\n{:?}",
//             db.state.online_users.read().await,
//             db.state.pending_messages.read().await
//         );
//
//         db.state
//             .message_add_vdb(EventMessage {
//                 // user one
//                 author: author.clone(), // user two
//                 targets: ["fsdgyfildsfdsh".to_string()].into(),
//                 item: EventEnum::MessageSend {
//                     id: rand(),
//                     author: author.clone(),
//                     content: "egg".to_string(),
//                     channel: "gfuoghlsduifhuguda".to_string(),
//                 },
//             })
//             .await;
//     }
// }

/// # action handler
/// this function reads incoming requests, depending on applicability and actions it adds events
/// to the memory database, this function is also responsible for sending establish
pub async fn action_handler(
    db: Data,
    user: User,
    events: Arc<RwLock<SplitSink<WebSocket, Message>>>,
    actions: Arc<RwLock<SplitStream<WebSocket>>>,
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
                you: user.id.clone(),
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
            ActionEnum::MessageSend { content, channel } => {
                // integrity check
                let Ok(Some(mut channel)) = db.get_channel_one(&user.id, &channel).await else {
                    continue;
                };
                if content.clone().replace([' ', '\n'], "").is_empty() {
                    continue;
                };
                channel.members.shift_remove(&user.id);
                db.state
                    .message_add_vdb(EventMessage {
                        author: user.id.clone(),
                        targets: channel.members,
                        item: EventEnum::MessageSend {
                            id: rand(),
                            author: user.id.clone(),
                            content,

                            channel: channel.id,
                        },
                    })
                    .await;
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
                        typing,
                        channel: channel.id,
                        user: user.id.clone(),
                    },
                });
            }
        }
    }
    // assumed disconnected
    db.state.user_offline(&user.id).await;
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
        if db.state.online_users.read().await.get(&user_id).is_none() {
            return;
        };
        let messages = db.state.get_message(&user_id).await;
        let Some(messages) = messages else {
            continue;
        };
        for item in messages {
            let _ = events.write().await.send(item.item.into()).await;
        }
    }
}
