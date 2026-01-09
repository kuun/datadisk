//! Setup handlers
//!
//! Implements database connection test and initialization endpoints

use axum::{extract::State, http::StatusCode, Json};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::config::DatabaseConfig;
use crate::db;
use crate::entity::user;
use crate::handlers::audit;
use crate::permission::PermissionEnforcer;
use crate::state::AppState;

/// Database connection test request
#[derive(Debug, Deserialize)]
pub struct TestDbRequest {
    #[serde(rename = "type")]
    pub db_type: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub username: String,
    pub password: String,
}

/// Setup response
#[derive(Debug, Serialize)]
pub struct SetupResponse {
    pub code: i32,
    pub message: String,
}

/// POST /api/setup/test-db
/// Test database connection
pub async fn test_db_connection(
    Json(req): Json<TestDbRequest>,
) -> (StatusCode, Json<SetupResponse>) {
    // Convert request to DatabaseConfig
    let port: u16 = req.port.parse().unwrap_or(5432);
    let config = DatabaseConfig {
        db_type: req.db_type,
        host: req.host,
        port,
        name: req.database,
        user: req.username,
        password: req.password,
    };

    tracing::info!("Testing database connection: {}:{}/{}", config.host, config.port, config.name);

    // Test connection
    match db::test_connection(&config).await {
        Ok(_) => (
            StatusCode::OK,
            Json(SetupResponse {
                code: 0,
                message: "连接成功".to_string(),
            }),
        ),
        Err(e) => {
            tracing::error!("Database connection test failed: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(SetupResponse {
                    code: 1,
                    message: format!("数据库连接失败: {}", e),
                }),
            )
        }
    }
}

/// POST /api/setup/init/db
/// Initialize database and save configuration
pub async fn init_db(
    State(state): State<AppState>,
    Json(req): Json<TestDbRequest>,
) -> (StatusCode, Json<SetupResponse>) {
    // Convert request to DatabaseConfig
    let port: u16 = req.port.parse().unwrap_or(5432);
    let config = DatabaseConfig {
        db_type: req.db_type,
        host: req.host,
        port,
        name: req.database,
        user: req.username,
        password: req.password,
    };

    tracing::info!("Initializing database: {}:{}/{}", config.host, config.port, config.name);

    // Test connection first
    if let Err(e) = db::test_connection(&config).await {
        tracing::error!("Database connection failed: {}", e);
        return (
            StatusCode::BAD_REQUEST,
            Json(SetupResponse {
                code: 1,
                message: format!("数据库连接失败: {}", e),
            }),
        );
    }

    // Initialize database (create tables)
    match db::init_database(&config).await {
        Ok(_) => {
            // Save database config to db.toml
            let db_path = state.config.config_dir.join("db.toml");
            let content = match toml::to_string_pretty(&config) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to serialize database config: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(SetupResponse {
                            code: 1,
                            message: format!("保存配置失败: {}", e),
                        }),
                    );
                }
            };

            if let Err(e) = std::fs::write(&db_path, content) {
                tracing::error!("Failed to save database config: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(SetupResponse {
                        code: 1,
                        message: format!("保存配置失败: {}", e),
                    }),
                );
            }

            (
                StatusCode::OK,
                Json(SetupResponse {
                    code: 0,
                    message: "数据库初始化成功".to_string(),
                }),
            )
        }
        Err(e) => {
            tracing::error!("Database initialization failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SetupResponse {
                    code: 1,
                    message: format!("初始化数据库失败: {}", e),
                }),
            )
        }
    }
}

/// Admin user initialization request
#[derive(Debug, Deserialize)]
pub struct InitUserRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

/// POST /api/setup/init/user
/// Create admin user and mark system as initialized
pub async fn init_user(
    State(state): State<AppState>,
    Json(req): Json<InitUserRequest>,
) -> (StatusCode, Json<SetupResponse>) {
    // Try to get existing db connection, or load from config and connect
    let db = if let Some(db) = state.get_db().await {
        db
    } else {
        // Load database config from db.toml and connect
        let db_toml_path = state.config.config_dir.join("db.toml");
        if !db_toml_path.exists() {
            return (
                StatusCode::BAD_REQUEST,
                Json(SetupResponse {
                    code: 1,
                    message: "请先初始化数据库".to_string(),
                }),
            );
        }

        let db_content = match std::fs::read_to_string(&db_toml_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to read db.toml: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(SetupResponse {
                        code: 1,
                        message: "读取数据库配置失败".to_string(),
                    }),
                );
            }
        };

        let db_config: crate::config::DatabaseConfig = match toml::from_str(&db_content) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to parse db.toml: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(SetupResponse {
                        code: 1,
                        message: "解析数据库配置失败".to_string(),
                    }),
                );
            }
        };

        let new_db = match db::init_database(&db_config).await {
            Ok(db) => db,
            Err(e) => {
                tracing::error!("Failed to connect to database: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(SetupResponse {
                        code: 1,
                        message: format!("数据库连接失败: {}", e),
                    }),
                );
            }
        };

        // Update state.db so future requests can use it
        state.set_db(new_db.clone()).await;
        new_db
    };

    let db = &db;

    // Check if user already exists
    let existing = user::Entity::find()
        .filter(user::Column::Username.eq(&req.username))
        .one(db)
        .await;

    match existing {
        Ok(Some(_)) => {
            // User exists, just mark as initialized
            tracing::info!("Admin user already exists: {}", req.username);
        }
        Ok(None) => {
            // Create admin user
            let hashed_password = match bcrypt::hash(&req.password, bcrypt::DEFAULT_COST) {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("Failed to hash password: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(SetupResponse {
                            code: 1,
                            message: "密码加密失败".to_string(),
                        }),
                    );
                }
            };

            let new_user = user::ActiveModel {
                username: Set(req.username.clone()),
                password: Set(hashed_password),
                full_name: Set(req.username.clone()),
                email: Set(req.email.clone()),
                department_id: Set(0),
                dept_name: Set(String::new()),
                status: Set(1),
                last_login: Set(0),
                permissions: Set(String::new()),
                ..Default::default()
            };

            if let Err(e) = new_user.insert(db).await {
                tracing::error!("Failed to create admin user: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(SetupResponse {
                        code: 1,
                        message: format!("创建管理员失败: {}", e),
                    }),
                );
            }

            // Create user directory
            let user_dir = state.config.root_dir.join(&req.username);
            if let Err(e) = std::fs::create_dir_all(&user_dir) {
                tracing::error!("Failed to create user directory: {}", e);
                // Don't fail the whole operation for this
            }

            tracing::info!("Admin user created: {}", req.username);

            // Initialize permission enforcer, create default roles, and assign admin role
            match PermissionEnforcer::new(
                db.clone(),
                state.config.casbin_conf.to_str().unwrap_or("./etc/casbin_model.conf"),
            ).await {
                Ok(perm_enforcer) => {
                    // Create default roles (admin, user)
                    if let Err(e) = perm_enforcer.ensure_default_roles().await {
                        tracing::error!("Failed to create default roles: {}", e);
                    }

                    // Assign admin role to first user
                    if let Err(e) = perm_enforcer.assign_user_role(&req.username, "admin").await {
                        tracing::error!("Failed to assign admin role: {}", e);
                    } else {
                        tracing::info!("Assigned admin role to user: {}", req.username);
                    }

                    // Store enforcer in app state
                    state.set_perm(perm_enforcer).await;
                }
                Err(e) => {
                    tracing::error!("Failed to initialize permission enforcer: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SetupResponse {
                    code: 1,
                    message: format!("数据库错误: {}", e),
                }),
            );
        }
    }

    // Initialize audit log service
    audit::service::init(db.clone());
    tracing::info!("Audit log service initialized");

    // Mark system as initialized (create sys_inited file)
    let inited_path = state.config.config_dir.join("sys_inited");
    if let Err(e) = std::fs::File::create(&inited_path) {
        tracing::error!("Failed to create sys_inited file: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SetupResponse {
                code: 1,
                message: "初始化失败".to_string(),
            }),
        );
    }

    tracing::info!("System initialization completed");

    (
        StatusCode::OK,
        Json(SetupResponse {
            code: 0,
            message: "初始化成功".to_string(),
        }),
    )
}
