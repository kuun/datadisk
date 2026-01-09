//! Task management system
//!
//! Provides background task management for file operations like copy/move

mod manager;

pub use manager::{ConflictPolicy, TaskNotification, TaskStatus, TASK_MANAGER};
