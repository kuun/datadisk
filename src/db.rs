use sea_orm::{
    ConnectionTrait, ConnectOptions, Database, DatabaseConnection, DbBackend, DbErr, Schema,
    Statement,
};
use sea_orm::sea_query::TableCreateStatement;
use std::time::Duration;
use tracing::info;

use crate::config::DatabaseConfig;
use crate::entity::{casbin_rule, department, file_access, file_info, group, group_user, op_log, user};

/// Initialize database connection and auto-migrate tables
pub async fn init_database(config: &DatabaseConfig) -> Result<DatabaseConnection, DbErr> {
    let database_url = config.connection_url();

    info!("Connecting to database: {}:{}/{}", config.host, config.port, config.name);

    let mut opt = ConnectOptions::new(&database_url);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(tracing::log::LevelFilter::Debug)
        .set_schema_search_path("public");

    let db = Database::connect(opt).await?;
    info!("Database connection established");

    // Auto-migrate tables
    auto_migrate(&db).await?;

    Ok(db)
}

/// Test database connection
pub async fn test_connection(config: &DatabaseConfig) -> Result<(), DbErr> {
    let database_url = config.connection_url();

    let mut opt = ConnectOptions::new(&database_url);
    opt.connect_timeout(Duration::from_secs(5));

    let db = Database::connect(opt).await?;
    db.ping().await?;

    Ok(())
}

/// Auto-migrate database tables (similar to GORM AutoMigrate)
async fn auto_migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    info!("Running auto-migration for all entities...");

    // Create tables in dependency order
    // 1. Independent tables first
    create_table_if_not_exists(db, backend, schema.create_table_from_entity(department::Entity)).await?;
    create_table_if_not_exists(db, backend, schema.create_table_from_entity(group::Entity)).await?;
    create_table_if_not_exists(db, backend, schema.create_table_from_entity(op_log::Entity)).await?;
    create_table_if_not_exists(db, backend, schema.create_table_from_entity(casbin_rule::Entity)).await?;

    // 2. Tables with foreign key dependencies
    create_table_if_not_exists(db, backend, schema.create_table_from_entity(user::Entity)).await?;
    create_table_if_not_exists(db, backend, schema.create_table_from_entity(file_info::Entity)).await?;
    create_table_if_not_exists(db, backend, schema.create_table_from_entity(group_user::Entity)).await?;
    create_table_if_not_exists(db, backend, schema.create_table_from_entity(file_access::Entity)).await?;

    // 3. Add missing columns to existing tables
    add_missing_columns(db, backend).await?;

    info!("Auto-migration completed successfully");
    Ok(())
}

/// Add missing columns to existing tables
async fn add_missing_columns(db: &DatabaseConnection, backend: DbBackend) -> Result<(), DbErr> {
    // Add permissions column to disk_user if not exists (legacy, kept for compatibility)
    add_column_if_not_exists(
        db,
        backend,
        "disk_user",
        "permissions",
        "VARCHAR(128) DEFAULT ''",
    ).await?;

    Ok(())
}

/// Add a column to a table if it doesn't exist
async fn add_column_if_not_exists(
    db: &DatabaseConnection,
    backend: DbBackend,
    table: &str,
    column: &str,
    column_def: &str,
) -> Result<(), DbErr> {
    // Check if column exists (PostgreSQL specific)
    let check_sql = format!(
        "SELECT column_name FROM information_schema.columns WHERE table_name = '{}' AND column_name = '{}'",
        table, column
    );

    let result = db.query_one(Statement::from_string(backend, check_sql)).await?;

    if result.is_none() {
        // Column doesn't exist, add it
        let alter_sql = format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            table, column, column_def
        );
        info!("Adding column {}.{}", table, column);
        db.execute(Statement::from_string(backend, alter_sql)).await?;
    }

    Ok(())
}

/// Create a table if it doesn't exist
async fn create_table_if_not_exists(
    db: &DatabaseConnection,
    backend: DbBackend,
    mut stmt: TableCreateStatement,
) -> Result<(), DbErr> {
    // Add IF NOT EXISTS to avoid errors when table already exists
    stmt.if_not_exists();

    let sql = backend.build(&stmt);

    db.execute(Statement::from_string(backend, sql.to_string())).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_url() {
        let config = DatabaseConfig {
            db_type: "postgres".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            name: "datadisk".to_string(),
            user: "postgres".to_string(),
            password: "secret".to_string(),
        };
        assert_eq!(
            config.connection_url(),
            "postgres://postgres:secret@localhost:5432/datadisk"
        );
    }
}
