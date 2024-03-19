#[cfg(feature = "server")]
pub mod db;
pub mod structures;
#[cfg(feature = "server")]
pub type Result<T> = std::result::Result<T, anyhow::Error>;
