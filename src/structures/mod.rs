use rand::random;

pub mod models;
pub mod websocket;

/// # rand
/// randomly generates u32 numbers and turns into strings, used for IDs
pub fn rand() -> String {
    random::<u32>().to_string()
}

/// # is_false
/// a simple check used by serde
pub fn is_false(i: &bool) -> bool {
    !*i
}
/// # is_none
/// a simple check used by serde
pub fn is_none<T>(i: &Option<T>) -> bool {
    i.is_none()
}
/// # is_none_bool
/// a simple check used by serde (is only false if Some(true)
pub fn is_none_bool(i: &Option<bool>) -> bool {
    !matches!(i, Some(true))
}
