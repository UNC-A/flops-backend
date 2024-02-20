use rand::random;

pub mod models;
pub mod websocket;
pub fn is_false(i: &bool) -> bool {
    !*i
}
pub fn is_none<T>(i: &Option<T>) -> bool {
    i.is_none()
}
pub fn rand() -> String {
    random::<u32>().to_string()
}
