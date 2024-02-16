use dotenv::dotenv;
use futures_util::{stream::StreamExt, SinkExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_tungstenite::tungstenite::Message;

//type read = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

type UserList = Arc<RwLock<Vec<String>>>;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let state: UserList = Arc::new(RwLock::new(vec![]));

    println!("starting WS server");

    let socket = TcpListener::bind(env::var("BIND").unwrap()).await.unwrap();
    checker(state.clone()).await;

    while let Ok((stream, address)) = socket.accept().await {
        //     tokio::spawn(connect(state.clone(), stream, address));
        connect(state.clone(), stream, address).await;
    }
}

async fn checker(list: UserList) {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        println!("Currently {} clients connected", list.read().await.len());
    }
}

async fn connect(state: UserList, stream: TcpStream, address: SocketAddr) {
    let stream = tokio_tungstenite::accept_async(stream).await.unwrap();

    // actions: user sending requests events: updating other users on events
    let (mut events, actions) = stream.split();

    events
        .send(EventEnum::Pong { data: None }.into())
        .await
        .unwrap();

    let _ = actions.for_each(|msg| {
        let msg = match msg {
            Ok(data) => data,
            Err(error) => {
                println!("{:?}", error);
                return futures_util::future::ready(());
            }
        };
        let data: ActionEnum = serde_json::from_slice(&msg.into_data()).unwrap();

        match data {
            ActionEnum::Ping { data } => {
                //   events.send(serde_json::to_vec(&EventEnum::Pong{data}).unwrap());
                let _ = events.send(EventEnum::Pong { data }.into());
            }
        }

        futures_util::future::ready(())
    });
}

impl From<EventEnum> for Message {
    fn from(value: EventEnum) -> Self {
        Message::from(serde_json::to_vec(&value).unwrap())
    }
}
impl From<ActionEnum> for Message {
    fn from(value: ActionEnum) -> Self {
        Message::from(serde_json::to_vec(&value).unwrap())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ActionEnum {
    Ping { data: Option<usize> },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum EventEnum {
    Pong { data: Option<usize> },
}
