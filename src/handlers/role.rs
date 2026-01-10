//! Role handlers
//!
//! Implements role CRUD operations using Casbin

use axum::{
    extract::{Query, State},
    response::Json,
    Extension,
};
use serde::{Deserialize, Serialize};

use crate::handlers::audit::service::log_operation;
use crate::middleware::auth::CurrentUser;
use crate::permission::{normalize_permissions, perm, RoleInfo};
use crate::routes::ApiResponse;
use crate::state::AppState;

// Operation types
const OP_CREATE_ROLE: &str = "创建角色";
const OP_DELETE_ROLE: &str = "删除角色";
const OP_UPDATE_ROLE: &str = "修改角色";
const OP_SUCCESS: &str = "成功";

/// Check if user has role management permission
fn can_manage_roles(user: &CurrentUser) -> bool {
    user.can_role()
}

/// Add role request
#[derive(Debug, Deserialize)]
pub struct AddRoleRequest {
    pub name: String,
    pub description: Option<String>,
    /// Comma-separated permissions or array of permissions
    pub permissions: String,
}

/// Update role request
#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: String,
    #[serde(rename = "oldName")]
    pub old_name: Option<String>,
    pub description: Option<String>,
    pub permissions: String,
}

/// Role response
#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub name: String,
    pub description: Option<String>,
    pub permissions: String,
    /// Permissions as array for frontend convenience
    #[serde(rename = "permissionList")]
    pub permission_list: Vec<String>,
}

impl From<RoleInfo> for RoleResponse {
    fn from(r: RoleInfo) -> Self {
        let permissions = r.permissions.join(",");
        Self {
            name: r.name,
            description: r.description,
            permissions,
            permission_list: r.permissions,
        }
    }
}

/// Query parameters for delete
#[derive(Debug, Deserialize)]
pub struct NameQuery {
    pub name: String,
}

/// POST /api/role/add
pub async fn add_role(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(req): Json<AddRoleRequest>,
) -> Json<ApiResponse<Option<RoleResponse>>> {
    // Permission check
    if !can_manage_roles(&user) {
        return Json(ApiResponse::error(403, "权限不足，仅管理员可创建角色"));
    }

    // Validate name
    if req.name.is_empty() {
        return Json(ApiResponse::error(400, "角色名称不能为空"));
    }
    if req.name.chars().count() > 32 {
        return Json(ApiResponse::error(400, "角色名称不能超过32个字符"));
    }

    // Get permission enforcer
    let perm_enforcer = match state.get_perm().await {
        Some(p) => p,
        None => return Json(ApiResponse::error(500, "权限系统未初始化")),
    };

    // Check if role already exists
    match perm_enforcer.get_role_permissions(&req.name).await {
        Ok(perms) if !perms.is_empty() => {
            return Json(ApiResponse::error(400, "角色名称已存在"));
        }
        _ => {}
    }

    // Normalize and validate permissions
    let perm_list_vec = normalize_permissions(&req.permissions);
    let perm_list: Vec<&str> = perm_list_vec.iter().map(String::as_str).collect();

    // Create role in Casbin
    if let Err(e) = perm_enforcer.create_role(&req.name, &perm_list).await {
        tracing::error!("Failed to create role: {}", e);
        return Json(ApiResponse::error(500, e.to_string()));
    }

    let op_desc = format!("角色名称: {}", req.name);
    log_operation(&user.username, OP_CREATE_ROLE, &op_desc, OP_SUCCESS, None);

    Json(ApiResponse::success(Some(RoleResponse {
        name: req.name,
        description: req.description,
        permissions: perm_list_vec.join(","),
        permission_list: perm_list_vec,
    })))
}

/// POST /api/role/delete
pub async fn delete_role(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Query(query): Query<NameQuery>,
) -> Json<ApiResponse<()>> {
    // Permission check
    if !can_manage_roles(&user) {
        return Json(ApiResponse::error(403, "权限不足，仅管理员可删除角色"));
    }

    // Prevent deletion of built-in roles
    if query.name == "admin" || query.name == "user" {
        return Json(ApiResponse::error(400, "不能删除内置角色"));
    }

    // Get permission enforcer
    let perm_enforcer = match state.get_perm().await {
        Some(p) => p,
        None => return Json(ApiResponse::error(500, "权限系统未初始化")),
    };

    // Check if role exists
    match perm_enforcer.get_role_permissions(&query.name).await {
        Ok(perms) if perms.is_empty() => {
            return Json(ApiResponse::error(404, "角色不存在"));
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, "internal error"));
        }
        _ => {}
    }

    // Delete role from Casbin
    if let Err(e) = perm_enforcer.delete_role(&query.name).await {
        tracing::error!("Failed to delete role: {}", e);
        return Json(ApiResponse::error(500, "删除失败"));
    }

    let op_desc = format!("角色名称: {}", query.name);
    log_operation(&user.username, OP_DELETE_ROLE, &op_desc, OP_SUCCESS, None);
    Json(ApiResponse::success_msg("success"))
}

/// POST /api/role/update
pub async fn update_role(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(req): Json<UpdateRoleRequest>,
) -> Json<ApiResponse<Option<RoleResponse>>> {
    // Permission check
    if !can_manage_roles(&user) {
        return Json(ApiResponse::error(403, "权限不足，仅管理员可修改角色"));
    }

    // Validate name
    if req.name.is_empty() {
        return Json(ApiResponse::error(400, "角色名称不能为空"));
    }
    if req.name.chars().count() > 32 {
        return Json(ApiResponse::error(400, "角色名称不能超过32个字符"));
    }

    // Get permission enforcer
    let perm_enforcer = match state.get_perm().await {
        Some(p) => p,
        None => return Json(ApiResponse::error(500, "权限系统未初始化")),
    };

    let old_name = req.old_name.as_deref().unwrap_or(&req.name);

    // Check if role exists
    match perm_enforcer.get_role_permissions(old_name).await {
        Ok(perms) if perms.is_empty() => {
            return Json(ApiResponse::error(404, "角色不存在"));
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, "internal error"));
        }
        _ => {}
    }

    // Normalize and validate permissions
    let perm_list_vec = normalize_permissions(&req.permissions);
    let perm_list: Vec<&str> = perm_list_vec.iter().map(String::as_str).collect();

    // Update role in Casbin
    if let Err(e) = perm_enforcer.update_role(old_name, &req.name, &perm_list).await {
        tracing::error!("Failed to update role: {}", e);
        return Json(ApiResponse::error(500, e.to_string()));
    }

    let op_desc = format!("角色名称: {}", req.name);
    log_operation(&user.username, OP_UPDATE_ROLE, &op_desc, OP_SUCCESS, None);

    Json(ApiResponse::success(Some(RoleResponse {
        name: req.name,
        description: req.description,
        permissions: perm_list_vec.join(","),
        permission_list: perm_list_vec,
    })))
}

/// Response format for role list
#[derive(Debug, Serialize)]
pub struct RoleListResponse {
    pub success: bool,
    pub data: Vec<RoleResponse>,
}

/// GET /api/role/list
pub async fn get_roles(
    State(state): State<AppState>,
    Extension(_user): Extension<CurrentUser>,
) -> Json<RoleListResponse> {
    // Get permission enforcer
    let perm_enforcer = match state.get_perm().await {
        Some(p) => p,
        None => {
            return Json(RoleListResponse {
                success: false,
                data: vec![],
            });
        }
    };

    match perm_enforcer.get_all_roles().await {
        Ok(roles) => {
            let response: Vec<RoleResponse> = roles.into_iter().map(|r| r.into()).collect();
            Json(RoleListResponse {
                success: true,
                data: response,
            })
        }
        Err(e) => {
            tracing::error!("Failed to get roles: {}", e);
            Json(RoleListResponse {
                success: false,
                data: vec![],
            })
        }
    }
}

/// Response for available permissions
#[derive(Debug, Serialize)]
pub struct PermissionsResponse {
    pub success: bool,
    pub data: Vec<PermissionInfo>,
}

#[derive(Debug, Serialize)]
pub struct PermissionInfo {
    pub key: String,
    pub name: String,
    pub description: String,
}

/// GET /api/role/permissions - Get list of available permissions
pub async fn get_available_permissions() -> Json<PermissionsResponse> {
    let permissions = vec![
        PermissionInfo {
            key: perm::FILE.to_string(),
            name: "文件管理".to_string(),
            description: "上传、下载、创建、删除文件".to_string(),
        },
        PermissionInfo {
            key: perm::CONTACTS.to_string(),
            name: "通讯录".to_string(),
            description: "管理用户、部门".to_string(),
        },
        PermissionInfo {
            key: perm::ROLE.to_string(),
            name: "角色管理".to_string(),
            description: "管理角色与角色权限".to_string(),
        },
        PermissionInfo {
            key: perm::GROUP.to_string(),
            name: "群组".to_string(),
            description: "管理群组及群组成员".to_string(),
        },
        PermissionInfo {
            key: perm::AUDIT.to_string(),
            name: "审计".to_string(),
            description: "查看操作日志".to_string(),
        },
    ];

    Json(PermissionsResponse {
        success: true,
        data: permissions,
    })
}
