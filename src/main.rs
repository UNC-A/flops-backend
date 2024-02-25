pub mod db;
pub mod structures;
pub type Result<T> = std::result::Result<T, anyhow::Error>;

use crate::db::vdb::EventMessage;
use crate::db::Data;
use crate::{
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

/// # Main.
/// Initializes MongoDB, VDB, TCP, Websocket and other services.
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

/// # App.
/// Mounts Websocket and Availability routes to web server.

fn app(db: Data) -> Router {
    Router::new()
        .route("/ws/", get(websocket).layer(Extension(db)))
        .route("/available/", get(available))
}
/// ## /available/
/// This route is used by external services to check if the server is online.
/// Simply put: Websocket cannot be easily cURLed.
async fn available() -> impl IntoResponse {
    (StatusCode::NO_CONTENT, "")
}

/// ## /ws/
/// This route contains the primary websocket server.
/// A valid session token is required for connecting to the server.
async fn websocket(
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
// todo very very bugged | this should be made into a client side service
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

/// # Action Handler
/// This function reads incoming requests, if applicable and valid the messages are added to the
/// volatile database and MongoDB.
///
/// Note: if all applicable clients are offline the message will NOT be sent to the volatile DB,
/// just MongoDB.
pub async fn action_handler(
    db: Data,
    user: User,
    events: Arc<RwLock<SplitSink<WebSocket, Message>>>,
    actions: Arc<RwLock<SplitStream<WebSocket>>>,
) {
    let (channels, users, messages) = db.establish(&user.id).await.unwrap();
    let _ = events
        .write()
        .await
        .send(
            EventEnum::Establish {
                channels,
                users,
                messages,
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
                let Ok(Some(mut channel)) = db.channel_get_one_accessible(&user.id, &channel).await
                else {
                    continue;
                };
                if content.clone().replace([' ', '\n'], "").is_empty() {
                    continue;
                };
                channel.members.shift_remove(&user.id);
                let item = EventEnum::MessageSend {
                    id: rand(),
                    author: user.id.clone(),
                    content,
                    channel: channel.id,
                };

                db.message_insert(item.clone().message_send().unwrap())
                    .await
                    .unwrap();
                db.state
                    .message_add_vdb(EventMessage {
                        author: user.id.clone(),
                        targets: channel.members,
                        item,
                    })
                    .await;
            }

            ActionEnum::TypeStatus { typing, channel } => {
                let Ok(Some(mut channel)) = db.channel_get_one_accessible(&user.id, &channel).await
                else {
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

/// # Event Handler.
/// This function reads from the volatile database for pending events, after a certain time interval
/// it sends all pending events to the specified user.
/// The performance for this model at scale is unknown, and may be migrated to redis at a later date.
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
