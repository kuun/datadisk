//! WebSocket module
//!
//! Provides real-time communication for task updates

mod hub;

pub use hub::serve_ws;
