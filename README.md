## Datadisk

Datadisk is a high-performance, enterprise-grade network disk. It ships a Rust/Axum backend with a React/Vite frontend and provides secure file management, permissions, audit trails, and collaboration-ready workflows.

[中文说明](README.zh-CN.md)

## Features

 - Users, departments, roles, and permission control (Casbin)
 - File and folder create/delete/move/copy/rename
 - Streaming uploads, single file downloads, and batch zip downloads
 - File preview and archive preview
 - Recent access, task management, and audit logs
 - WebSocket notifications
 - OnlyOffice online editing (optional)

## Quick Start

### Requirements

- Rust 1.70+ (via rustup)
- Node.js 18+
- PostgreSQL 12+

### Backend

1. Copy and edit config:
   ```bash
   cp etc/datadisk_example.toml etc/datadisk.toml
   ```
2. Update `etc/datadisk.toml`:
   - `addr`: server bind address
   - `root_dir`: file storage root
   - `config_dir`: writable config dir for `db.toml` and `sys_inited`
3. Start the backend:
   ```bash
   cargo run -- -config etc/datadisk.toml
   ```
4. Open the setup page to initialize DB and admin user:
   - `http://localhost:8080/setup.html`

### Frontend

```bash
cd webapp
npm install
npm run dev
```

The dev server proxies `http://127.0.0.1:8080`. Open `http://localhost:5173`.

### Build

```bash
cd webapp
npm run build
```

Assets are emitted to `webapp/dist`, which the backend serves as static files.

## Development

### Backend

- Run locally:
  ```bash
  cargo run -- -config etc/datadisk.toml
  ```
- Run tests:
  ```bash
  cargo test
  ```
- Adjust logging:
  ```bash
  RUST_LOG=debug cargo run -- -config etc/datadisk.toml
  ```

### Frontend

- Start dev server:
  ```bash
  cd webapp
  npm run dev
  ```
- Lint:
  ```bash
  npm run lint
  ```

## Structure

- `src/`: backend code (Axum + SeaORM)
- `webapp/`: frontend code (React + Vite)
- `etc/`: configuration and init files
- `testdir/`: sample data and config for local testing

## License

Apache License 2.0. See `LICENSE`.
