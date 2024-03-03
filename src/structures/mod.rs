#[cfg(feature = "server")]
use rand::random;

pub mod models;
pub mod websocket;
#[cfg(feature = "server")]
/// Randomly generates u32 numbers and turns into strings, used for IDs
pub fn rand() -> String {
    random::<u32>().to_string()
}

/// A simple check used by serde | Default data such as 'false', 'None' or 'Empty' are not sent
/// to save data.

/// Only sends if condition == true
pub fn is_false(i: &bool) -> bool {
    !*i
}

/// Only sends data if condition == Some(true).
pub fn is_none_bool(i: &Option<bool>) -> bool {
    !matches!(i, Some(true))
}
