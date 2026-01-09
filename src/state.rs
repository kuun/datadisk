use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::config::Config;
use crate::permission::PermissionEnforcer;

/// WebSocket notification message
#[derive(Clone, Debug)]
pub struct WsNotification {
    pub user_id: i64,
    pub message: String,
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool (None if system not initialized, can be set at runtime)
    pub db: Arc<RwLock<Option<DatabaseConnection>>>,
    /// Permission enforcer (None if system not initialized)
    pub perm: Arc<RwLock<Option<PermissionEnforcer>>>,
    /// Application configuration
    pub config: Arc<Config>,
    /// WebSocket notification sender
    pub ws_sender: broadcast::Sender<WsNotification>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        db: Option<DatabaseConnection>,
        perm: Option<PermissionEnforcer>,
        config: Config,
    ) -> Self {
        let (ws_sender, _) = broadcast::channel(1000);

        Self {
            db: Arc::new(RwLock::new(db)),
            perm: Arc::new(RwLock::new(perm)),
            config: Arc::new(config),
            ws_sender,
        }
    }

    /// Check if system is initialized and database is available
    /// Note: We check the file directly to reflect runtime changes during setup
    pub async fn is_initialized(&self) -> bool {
        let inited_path = self.config.config_dir.join("sys_inited");
        self.db.read().await.is_some() && inited_path.exists()
    }

    /// Get database connection, returns None if not initialized
    pub async fn get_db(&self) -> Option<DatabaseConnection> {
        self.db.read().await.clone()
    }

    /// Get database connection, panics if not initialized
    /// This is the main method used by handlers
    pub async fn db(&self) -> DatabaseConnection {
        self.db
            .read()
            .await
            .clone()
            .expect("Database not initialized")
    }

    /// Set database connection (used during setup)
    pub async fn set_db(&self, db: DatabaseConnection) {
        *self.db.write().await = Some(db);
    }

    /// Get permission enforcer, returns None if not initialized
    pub async fn get_perm(&self) -> Option<PermissionEnforcer> {
        self.perm.read().await.clone()
    }

    /// Set permission enforcer (used during setup)
    pub async fn set_perm(&self, perm: PermissionEnforcer) {
        *self.perm.write().await = Some(perm);
    }

    /// Send notification to a specific user via WebSocket
    pub fn notify_user(&self, user_id: i64, message: impl Into<String>) {
        let notification = WsNotification {
            user_id,
            message: message.into(),
        };
        // Ignore send errors (no receivers is fine)
        let _ = self.ws_sender.send(notification);
    }

    /// Subscribe to WebSocket notifications
    pub fn subscribe(&self) -> broadcast::Receiver<WsNotification> {
        self.ws_sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_notification() {
        let notification = WsNotification {
            user_id: 1,
            message: "test".to_string(),
        };
        assert_eq!(notification.user_id, 1);
        assert_eq!(notification.message, "test");
    }
}
