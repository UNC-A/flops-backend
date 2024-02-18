mod db;
pub mod structures;
use crate::structures::websocket::actions::ActionEnum;
use crate::structures::websocket::events::EventEnum;
use dotenv::dotenv;
use futures_util::stream::SplitSink;
use futures_util::{stream::StreamExt, SinkExt};
use rand::random;
use std::collections::{HashMap, HashSet};
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

type UserList = HashMap<u32, String>;

#[derive(Clone, Debug)]
pub struct MessageCount {
    pub message: String,
    pub sent_by: u32,
    pub ids: HashSet<u32>,
}

#[derive(Clone, Default, Debug)]
pub struct States {
    pub messages: Vec<MessageCount>,
    pub user_list: UserList,
}

impl States {
    pub fn user_remove(&mut self, id: u32) {
        self.user_list.remove(&id);
        self.messages = self
            .messages
            .clone()
            .into_iter()
            .map(|mut a| {
                a.ids.remove(&id);
                a
            })
            .collect()
    }
    pub fn user_add(&mut self, id: u32, address: String) {
        self.user_list.insert(id, address);
        self.messages = self
            .messages
            .clone()
            .into_iter()
            .map(|mut a| {
                if id != a.sent_by {
                    a.ids.insert(id);
                };
                a
            })
            .collect()
    }
    // sends a collection of messages to send, and removes id from each of them
    pub fn do_send(&mut self, id: u32) -> Vec<String> {
        // applicable messages
        let data = self
            .messages
            .clone()
            .into_iter()
            .filter(|a| a.sent_by != id)
            .map(|a| a.message)
            .collect();
        // remove ids
        self.messages = self
            .messages
            .clone()
            .into_iter()
            .map(|mut a| {
                a.ids.remove(&id);
                a
            })
            .filter(|a| !a.ids.is_empty())
            .collect();
        data
    }

    pub fn message_add(&mut self, id: u32, message: String) {
        self.messages.push(MessageCount {
            message,
            sent_by: id,
            ids: self
                .user_list
                .clone()
                .into_iter()
                .filter_map(
                    |(iter_id, _)| {
                        if iter_id != id {
                            Some(iter_id)
                        } else {
                            None
                        }
                    },
                )
                .collect(),
        })
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // in memory database
    let state: Arc<RwLock<States>> = Default::default();

    println!("starting WS server");

    let socket = TcpListener::bind(env::var("BIND").unwrap()).await.unwrap();
    tokio::spawn(checker(state.clone()));
    while let Ok((stream, address)) = socket.accept().await {
        tokio::spawn(connect(state.clone(), stream, address));
    }
}

async fn checker(list: Arc<RwLock<States>>) {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        println!("{:?}", list.read().await.clone(),);
    }
}

async fn connect(state: Arc<RwLock<States>>, stream: TcpStream, address: SocketAddr) {
    let id = random();
    state.write().await.user_add(id, address.to_string());

    let stream = tokio_tungstenite::accept_async(stream).await.unwrap();

    // actions: user sending requests events: updating other users on events
    let (events, mut actions) = stream.split();

    let events = Arc::new(RwLock::new(events));
    tokio::spawn(connect_events(events.clone(), id, state.clone()));
    loop {
        if let Some(Ok(msg)) = actions.next().await {
            let Ok(data) = serde_json::from_str::<ActionEnum>(&msg.to_string()) else {
                println!("data schema does not match, defaulting to message");
                state.write().await.message_add(id, msg.to_string());
                continue;
            };

            if msg.is_binary() {
                println!("YIPEEE BINARY");
                continue;
            };

            match data {
                ActionEnum::Ping { data } => {
                    println!("poong");
                    events
                        .write()
                        .await
                        .send(EventEnum::Pong { data }.into())
                        .await
                        .unwrap();
                }
                _ => {}
            }
        } else {
            println!("connection closed");
            state.write().await.user_remove(id);
            break;
        }
    }
}

async fn connect_events(
    events: Arc<RwLock<SplitSink<WebSocketStream<TcpStream>, Message>>>,
    id: u32,
    state: Arc<RwLock<States>>,
) {
    loop {
        tokio::time::sleep(Duration::from_millis(1)).await;

        let data = state.write().await.do_send(id).clone();

        for message in data {
            events.write().await.send(Message::Text(message)).await;
        }
    }
}
