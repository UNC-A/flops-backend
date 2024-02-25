/// # Websocket
/// The majority of connectivity happens over WS, Websocket offers superior performance as it
/// does not require cold starts.

/// ## Actions
/// Client -> Server
pub mod actions;
/// ## Events
/// Server -> Clients
pub mod events;
