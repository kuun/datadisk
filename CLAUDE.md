# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Datadisk is a network disk management system built with Rust (backend) and React (frontend). It provides file management capabilities with WebDAV support, user authentication, department/group management, and real-time features through WebSocket connections.

## Development Commands

### Backend (Rust)
- **Build**: `cargo build` or `cargo build --release`
- **Run**: `cargo run` or `./target/release/datadisk`
- **Test**: `cargo test`
- **Dependencies**: `cargo update` to update dependencies

### Frontend (React)
Located in `webapp/` directory:
- **Development**: `npm run dev` (serves on 0.0.0.0 with proxy to backend)
- **Build**: `npm run build` (outputs to `webapp/dist/`)
- **Lint**: `npm run lint`
- **Preview**: `npm run preview`

## Architecture

### Backend Structure (src/)
- **main.rs**: Application entry point with Axum server initialization
- **config.rs**: Configuration management using TOML files
- **routes/**: HTTP routing and API endpoint definitions
- **middleware/**: Authentication middleware with permission checking
- **entity/**: SeaORM entity definitions (User, Department, Role, FileInfo, etc.)
- **handlers/**: Request handlers for all API endpoints
- **file/**: Core file operations, upload/download, archive handling
- **webdav/**: WebDAV protocol implementation
- **message/**: WebSocket hub for real-time communication
- **taskmgr/**: Background task management (copy operations, etc.)
- **permission.rs**: Casbin permission enforcer

### Frontend Structure
- **Multiple entry points**: index.html (main app), editor.html (document editing), setup.html (initial setup)
- **React** with hooks, React Router for navigation
- **shadcn/ui** component library with Radix UI primitives
- **Tailwind CSS** for styling
- **Key stores**: fileStore, contacts, groups, tasks (via React Context)
- **Components**: File management, user/group management, task monitoring, document editing

### Key Integrations
- **OnlyOffice** document editing integration
- **WebDAV** server for external client access
- **Real-time communication** via WebSocket for task updates
- **Multi-format archive support** (7z, rar, zip)
- **PostgreSQL** as primary database with SeaORM

## Database Configuration
- Uses PostgreSQL with SeaORM
- Table prefix: `disk_`
- Models auto-migrate on initialization
- Configuration stored in TOML format (`etc/db.toml`)

## Development Notes
- Backend serves frontend static files from `webapp/dist/`
- API endpoints prefixed with `/api/`
- WebDAV accessible at `/api/webdav/*`
- Frontend dev server proxies API calls to `127.0.0.1:8080`
- WebSocket endpoint at `/api/ws`
- Setup wizard available for initial configuration

## Git Commit Guidelines
- Commit messages should not contain Claude or AI-related information
- Keep commit messages concise and descriptive

## File Operations
The system supports:
- File upload with chunked/resumable uploads
- File download with range support
- Copy/move operations with conflict resolution
- Archive preview (zip, rar, 7z)
- Directory operations (create, delete, list)

## Permission Management (RBAC)

The system uses Role-Based Access Control with role inheritance support.

### Architecture
```
┌─────────────┐     ┌─────────────┐     ┌──────────────────┐
│    Role     │────▶│ casbin_rule │◀────│ PermissionEnforcer│
│ (disk_role) │     │  (fallback) │     │   (权限检查)      │
└─────────────┘     └─────────────┘     └──────────────────┘
       │
       ▼
┌─────────────┐     ┌─────────────┐
│ Department  │────▶│    User     │
│ (role_id)   │     │  (role_id)  │
└─────────────┘     └─────────────┘
```

### Permission Types
- `file` - File management (upload, download, create, delete)
- `contacts` - User and department management
- `group` - Group management
- `audit` - Audit log access

### Default Roles
| Role | Permissions | Description |
|------|-------------|-------------|
| admin | file,contacts,group,audit | 系统管理员，拥有所有权限 |
| user | file,group | 普通用户，拥有文件和群组权限 |

### Permission Resolution (Inheritance Chain)
```
1. user.role_id set       → use user's role permissions
2. department.role_id set → use department's role permissions
3. parent department      → inherit from parent (recursive)
4. Casbin policies        → fallback for backward compatibility
```

### Key Files
- `src/entity/role.rs` - Role entity definition
- `src/handlers/role.rs` - Role CRUD handlers
- `src/permission.rs` - Casbin enforcer (fallback)
- `src/middleware/auth.rs` - Permission resolution logic
- `src/db.rs` - Default roles creation

### API Endpoints
```
# Role Management
GET    /api/role/list         - List all roles
GET    /api/role/permissions  - Get available permission types
POST   /api/role/add          - Create role
POST   /api/role/update       - Update role
POST   /api/role/delete       - Delete role

# User/Department role assignment
POST   /api/user/add          - Include roleId param
POST   /api/user/update       - Include roleId param
POST   /api/department/add    - Include roleId param
POST   /api/department/update - Include roleId param
```

### Role Table Schema (disk_role)
| Column | Type | Description |
|--------|------|-------------|
| id | BIGINT | Primary key |
| name | VARCHAR(32) | Unique role name |
| description | VARCHAR(128) | Role description |
| permissions | VARCHAR(128) | Comma-separated: "file,contacts,group,audit" |
| created_at | BIGINT | Unix timestamp |
| updated_at | BIGINT | Unix timestamp |

### First User Setup
The first user created during setup is automatically assigned the `admin` role.