//! Datadisk - A network disk management system
//!
//! This crate provides the core functionality for the Datadisk file management system,
//! including file operations, user management, and real-time task notifications.

// Allow dead code for reserved/future-use structures in entity and error modules
#![allow(dead_code)]

pub mod config;
pub mod db;
pub mod entity;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod permission;
pub mod routes;
pub mod state;
pub mod task;
pub mod ws;

// Re-export commonly used types
pub use config::Config;
pub use state::AppState;
