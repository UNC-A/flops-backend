use rand::random;

pub mod websocket;


pub fn rand() -> String {
    random::<u32>().to_string()
}