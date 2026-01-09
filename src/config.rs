use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Server address (e.g., "0.0.0.0:8080")
    #[serde(default = "default_addr")]
    pub addr: String,
    /// Root directory for file storage
    #[serde(default = "default_root_dir")]
    pub root_dir: PathBuf,
    /// Casbin configuration file path
    #[serde(default = "default_casbin_conf")]
    pub casbin_conf: PathBuf,
    /// Configuration directory
    #[serde(default = "default_config_dir")]
    pub config_dir: PathBuf,
    /// Whether the system has been initialized (computed, not from file)
    #[serde(skip)]
    pub initialized: bool,
    /// Path to the initialization marker file (computed, not from file)
    #[serde(skip)]
    pub inited_path: PathBuf,
    /// Logging configuration
    #[serde(default)]
    pub log: LogConfig,
    /// OnlyOffice configuration
    #[serde(default)]
    pub doc: DocConfig,
    /// Database configuration (loaded from separate file)
    #[serde(skip)]
    pub database: DatabaseConfig,
    /// Maximum upload file size in bytes (default: 10GB)
    #[serde(default = "default_max_upload_size")]
    pub max_upload_size: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogConfig {
    /// Log level: trace, debug, info, warn, error
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DocConfig {
    /// OnlyOffice document server URL
    #[serde(default)]
    pub doc_server_url: String,
    /// OnlyOffice secret key
    #[serde(default)]
    pub doc_secret: String,
    /// Datadisk server URL (for callbacks)
    #[serde(default)]
    pub datadisk_url: String,
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// Database type (postgres)
    #[serde(default = "default_db_type", rename = "type")]
    pub db_type: String,
    /// Database host
    #[serde(default = "default_db_host")]
    pub host: String,
    /// Database port
    #[serde(default = "default_db_port")]
    pub port: u16,
    /// Database name
    #[serde(default = "default_db_name", rename = "database")]
    pub name: String,
    /// Database user
    #[serde(default = "default_db_user", rename = "username")]
    pub user: String,
    /// Database password
    #[serde(default)]
    pub password: String,
}

// Default value functions
fn default_addr() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_root_dir() -> PathBuf {
    PathBuf::from("./data")
}

// Note: The casbin_conf path should be configured in datadisk.toml
// Example: casbin_conf = "./etc/casbin_model.conf"
fn default_casbin_conf() -> PathBuf {
    PathBuf::from("./etc/casbin_model.conf")
}

fn default_config_dir() -> PathBuf {
    PathBuf::from("./etc")
}

fn default_db_type() -> String {
    "postgres".to_string()
}

fn default_db_host() -> String {
    "localhost".to_string()
}

fn default_db_port() -> u16 {
    5432
}

fn default_db_name() -> String {
    "datadisk".to_string()
}

fn default_db_user() -> String {
    "postgres".to_string()
}

fn default_max_upload_size() -> usize {
    10 * 1024 * 1024 * 1024 // 10GB
}

impl Default for Config {
    fn default() -> Self {
        Self {
            addr: default_addr(),
            root_dir: default_root_dir(),
            casbin_conf: default_casbin_conf(),
            config_dir: default_config_dir(),
            initialized: false,
            inited_path: PathBuf::from("./etc/.inited"),
            log: LogConfig::default(),
            doc: DocConfig::default(),
            database: DatabaseConfig::default(),
            max_upload_size: default_max_upload_size(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_type: default_db_type(),
            host: default_db_host(),
            port: default_db_port(),
            name: default_db_name(),
            user: default_db_user(),
            password: String::new(),
        }
    }
}

impl DatabaseConfig {
    /// Generate database connection URL
    pub fn connection_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.name
        )
    }
}

impl Config {
    /// Load configuration from TOML file
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;

        // Check if initialized (sys_inited file exists)
        config.inited_path = config.config_dir.join("sys_inited");
        config.initialized = config.inited_path.exists();

        // Load database config from separate db.toml file
        if config.initialized {
            let db_toml_path = config.config_dir.join("db.toml");
            if db_toml_path.exists() {
                let db_content = std::fs::read_to_string(&db_toml_path)?;
                config.database = toml::from_str(&db_content)?;
            }
        }

        Ok(config)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.addr, "0.0.0.0:8080");
        assert!(!config.initialized);
    }

    #[test]
    fn test_database_url() {
        let db = DatabaseConfig {
            db_type: "postgres".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            name: "testdb".to_string(),
            user: "user".to_string(),
            password: "pass".to_string(),
        };
        assert_eq!(db.connection_url(), "postgres://user:pass@localhost:5432/testdb");
    }

    #[test]
    fn test_toml_parse() {
        let toml_str = r#"
            addr = "127.0.0.1:9000"
            root_dir = "/data"

            [doc]
            doc_server_url = "http://localhost:8082"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.addr, "127.0.0.1:9000");
        assert_eq!(config.root_dir, PathBuf::from("/data"));
        assert_eq!(config.doc.doc_server_url, "http://localhost:8082");
    }
}
