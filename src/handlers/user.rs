//! User handlers
//!
//! Implements user CRUD operations

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
    Extension,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};

use crate::entity::user;
use crate::handlers::audit::service::log_operation;
use crate::middleware::auth::CurrentUser;
use crate::middleware::DbConn;
use crate::permission::normalize_permissions;
use crate::routes::ApiResponse;
use crate::state::AppState;

// Operation types (matching Go version)
const OP_CREATE_USER: &str = "创建用户信息";
const OP_DELETE_USER: &str = "删除用户信息";
const OP_UPDATE_USER: &str = "修改用户信息";
const OP_QUERY_USER: &str = "查询用户信息";
const OP_ENABLE_USER: &str = "启用用户";
const OP_DISABLE_USER: &str = "禁用用户";
const OP_UPDATE_PASSWORD: &str = "修改密码";
const OP_SUCCESS: &str = "成功";
const OP_FAILED: &str = "失败";

/// Check if user has contacts permission (for user management)
fn can_manage_users(user: &CurrentUser) -> bool {
    user.can_contacts()
}

/// Response with boolean code (matching Go version)
#[derive(Debug, Serialize)]
pub struct BoolCodeResponse {
    pub code: bool,
    pub message: String,
}

impl BoolCodeResponse {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            code: true,
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            code: false,
            message: message.into(),
        }
    }
}

/// Add user request
#[derive(Debug, Deserialize)]
pub struct AddUserRequest {
    pub username: String,
    pub password: String,
    #[serde(rename = "fullName")]
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "departmentId")]
    pub department_id: i64,
    /// Role name (e.g., "admin", "user")
    pub role: Option<String>,
    pub quota: Option<String>,
    pub permissions: Option<String>,
}

/// Update user request
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub id: i64,
    pub username: String,
    pub password: Option<String>,
    #[serde(rename = "fullName")]
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "departmentId")]
    pub department_id: i64,
    #[serde(rename = "deptName")]
    pub dept_name: Option<String>,
    /// Role name (e.g., "admin", "user")
    pub role: Option<String>,
    pub quota: Option<String>,
    pub permissions: Option<String>,
}

/// Delete user request (array of users)
#[derive(Debug, Deserialize)]
pub struct DeleteUserItem {
    pub id: i64,
    pub username: String,
    #[serde(rename = "departmentId")]
    pub department_id: i64,
}

/// User response
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    #[serde(rename = "fullName")]
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "lastLogin")]
    pub last_login: i32,
    #[serde(rename = "departmentId")]
    pub department_id: i64,
    #[serde(rename = "deptName")]
    pub dept_name: String,
    /// User's role name (from Casbin)
    pub role: Option<String>,
    pub status: i32,
    pub quota: Option<String>,
    #[serde(rename = "effectiveQuota")]
    pub effective_quota: Option<String>,
    pub permissions: String,
    #[serde(rename = "permissionList")]
    pub permission_list: Vec<String>,
}

impl UserResponse {
    /// Create from user model with role from Casbin
    pub fn from_model_with_role(
        m: user::Model,
        role: Option<String>,
        direct_permissions: Vec<String>,
        effective_quota: Option<String>,
    ) -> Self {
        let permissions = direct_permissions.join(",");
        Self {
            id: m.id,
            username: m.username,
            full_name: m.full_name,
            phone: m.phone,
            email: m.email,
            last_login: m.last_login,
            department_id: m.department_id,
            dept_name: m.dept_name,
            role,
            status: m.status,
            quota: m.quota,
            effective_quota,
            permissions,
            permission_list: direct_permissions,
        }
    }
}

impl From<user::Model> for UserResponse {
    fn from(m: user::Model) -> Self {
        Self::from_model_with_role(m, None, Vec::new(), None)
    }
}

/// Query parameters
#[derive(Debug, Deserialize)]
pub struct DepartmentIdQuery {
    #[serde(rename = "departmentId")]
    pub department_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct UsernameQuery {
    pub username: String,
}

/// Enable/disable user request
#[derive(Debug, Deserialize)]
pub struct UserStatusItem {
    pub id: i64,
    pub username: String,
}

/// Change password request (user changes their own password)
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    #[serde(rename = "oldPassword")]
    pub old_password: String,
    #[serde(rename = "newPassword")]
    pub new_password: String,
}

/// Reset password request (admin resets user password)
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub id: i64,
    pub username: String,
    pub password: String,
}

/// POST /api/user/add
pub async fn add_user(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<AddUserRequest>,
) -> Json<BoolCodeResponse> {
    // Permission check: only admin can add users
    if !can_manage_users(&current_user) {
        return Json(BoolCodeResponse::error("权限不足，仅管理员可添加用户"));
    }

    let existing = user::Entity::find()
        .filter(user::Column::Username.eq(&req.username))
        .one(&*db)
        .await;

    match existing {
        Ok(Some(_)) => return Json(BoolCodeResponse::error("用户名已存在")),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(BoolCodeResponse::error("internal error"));
        }
        Ok(None) => {}
    }

    let hashed_password = match bcrypt::hash(&req.password, 12) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash password: {}", e);
            return Json(BoolCodeResponse::error("密码加密失败"));
        }
    };

    let dept_name = get_department_name(&*db, req.department_id).await;
    let quota = normalize_quota(req.quota.clone());

    let new_user = user::ActiveModel {
        username: Set(req.username.clone()),
        password: Set(hashed_password),
        full_name: Set(req.full_name),
        phone: Set(req.phone),
        email: Set(req.email),
        department_id: Set(req.department_id),
        dept_name: Set(dept_name.clone()),
        status: Set(0),
        quota: Set(quota),
        last_login: Set(0),
        ..Default::default()
    };

    match new_user.insert(&*db).await {
        Ok(_) => {
            // Create user directory (same as Go version)
            let user_dir = state.config.root_dir.join(&req.username);
            if let Err(e) = tokio::fs::create_dir_all(&user_dir).await {
                tracing::error!("Failed to create user directory: {}", e);
                return Json(BoolCodeResponse::error("创建用户目录失败"));
            }

            if let Some(perm_enforcer) = state.get_perm().await.as_ref() {
                // Assign role via Casbin if specified
                if let Some(role) = &req.role {
                    if let Err(e) = perm_enforcer.set_user_role(&req.username, Some(role)).await {
                        tracing::error!("Failed to assign role: {}", e);
                    }
                }
                // Assign department for permission inheritance
                if let Err(e) = perm_enforcer.set_user_department(&req.username, req.department_id).await {
                    tracing::error!("Failed to assign department: {}", e);
                }
                // Set direct user permissions if provided
                if let Some(perms) = req.permissions.as_deref() {
                    let perm_list = normalize_permissions(perms);
                    let perm_refs: Vec<&str> = perm_list.iter().map(String::as_str).collect();
                    if let Err(e) = perm_enforcer.set_permissions(&req.username, &perm_refs).await {
                        tracing::error!("Failed to set user permissions: {}", e);
                    }
                }
            }

            // Log operation
            let op_desc = format!("所属部门: {}, 用户名: {}", dept_name, req.username);
            log_operation(&current_user.username, OP_CREATE_USER, &op_desc, OP_SUCCESS, None);
            Json(BoolCodeResponse::success("success"))
        }
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            Json(BoolCodeResponse::error(e.to_string()))
        }
    }
}

/// POST /api/user/delete
pub async fn delete_user(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(users): Json<Vec<DeleteUserItem>>,
) -> Json<BoolCodeResponse> {
    // Permission check: only admin can delete users
    if !can_manage_users(&current_user) {
        return Json(BoolCodeResponse::error("权限不足，仅管理员可删除用户"));
    }

    use crate::entity::file_info;

    let mut success_count = 0;
    let mut error_count = 0;

    // Get department name for first user (same as Go version)
    let dept_name = if !users.is_empty() {
        get_department_name(&*db, users[0].department_id).await
    } else {
        String::new()
    };

    for u in users {
        let op_desc = format!("所属部门: {}, 用户名: {}", dept_name, u.username);
        match user::Entity::delete_by_id(u.id).exec(&*db).await {
            Ok(_) => {
                success_count += 1;

                // Delete user's file info from database
                if let Err(e) = file_info::Entity::delete_many()
                    .filter(file_info::Column::Username.eq(&u.username))
                    .exec(&*db)
                    .await
                {
                    tracing::error!("Failed to delete file info for user {}: {}", u.username, e);
                }

                // Delete user directory
                let user_dir = state.config.root_dir.join(&u.username);
                if let Err(e) = tokio::fs::remove_dir_all(&user_dir).await {
                    // Ignore error if directory doesn't exist
                    if e.kind() != std::io::ErrorKind::NotFound {
                        tracing::error!("Failed to delete user directory {}: {}", u.username, e);
                    }
                }
                // Log success
                log_operation(&current_user.username, OP_DELETE_USER, &op_desc, OP_SUCCESS, None);
            }
            Err(e) => {
                tracing::error!("Failed to delete user {}: {}", u.username, e);
                error_count += 1;
                // Log failure
                log_operation(&current_user.username, OP_DELETE_USER, &op_desc, OP_FAILED, None);
            }
        }
    }

    let message = format!("成功删除{}个用户, 失败{}个", success_count, error_count);
    Json(BoolCodeResponse::success(message))
}

/// POST /api/user/update
pub async fn update_user(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<UpdateUserRequest>,
) -> Json<BoolCodeResponse> {
    // Permission check: only admin can update other users
    if !can_manage_users(&current_user) && req.id != current_user.id {
        return Json(BoolCodeResponse::error("权限不足，仅管理员可修改其他用户"));
    }

    let existing = user::Entity::find_by_id(req.id).one(&*db).await;

    let old_user = match existing {
        Ok(Some(u)) => u,
        Ok(None) => return Json(BoolCodeResponse::error("用户不存在")),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(BoolCodeResponse::error("查询用户失败"));
        }
    };

    let password = if let Some(new_pwd) = req.password {
        if !new_pwd.is_empty() {
            match bcrypt::hash(&new_pwd, 12) {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("Failed to hash password: {}", e);
                    return Json(BoolCodeResponse::error("密码加密失败"));
                }
            }
        } else {
            old_user.password.clone()
        }
    } else {
        old_user.password.clone()
    };

    let dept_name = get_department_name(&*db, req.department_id).await;
    let quota = match req.quota.clone() {
        Some(q) => normalize_quota(Some(q)),
        None => old_user.quota.clone(),
    };

    let update_model = user::ActiveModel {
        id: Set(req.id),
        username: Set(req.username.clone()),
        password: Set(password),
        full_name: Set(req.full_name),
        phone: Set(req.phone),
        email: Set(req.email),
        department_id: Set(req.department_id),
        dept_name: Set(req.dept_name.unwrap_or(old_user.dept_name)),
        status: Set(old_user.status),
        quota: Set(quota),
        last_login: Set(old_user.last_login),
        permissions: Set(old_user.permissions), // Preserve existing permissions
    };

    match update_model.update(&*db).await {
        Ok(_) => {
            if let Some(perm_enforcer) = state.get_perm().await.as_ref() {
                // Update role via Casbin
                if let Err(e) = perm_enforcer.set_user_role(&req.username, req.role.as_deref()).await {
                    tracing::error!("Failed to update role: {}", e);
                }
                // Update department for permission inheritance
                if let Err(e) = perm_enforcer.set_user_department(&req.username, req.department_id).await {
                    tracing::error!("Failed to update department: {}", e);
                }
                // Update direct permissions if provided
                if let Some(perms) = req.permissions.as_deref() {
                    let perm_list = normalize_permissions(perms);
                    let perm_refs: Vec<&str> = perm_list.iter().map(String::as_str).collect();
                    if let Err(e) = perm_enforcer.set_permissions(&req.username, &perm_refs).await {
                        tracing::error!("Failed to update user permissions: {}", e);
                    }
                }
            }

            // Log operation
            let op_desc = format!("所属部门: {}, 用户名: {}", dept_name, req.username);
            log_operation(&current_user.username, OP_UPDATE_USER, &op_desc, OP_SUCCESS, None);
            Json(BoolCodeResponse::success("success"))
        }
        Err(e) => {
            tracing::error!("Failed to update user: {}", e);
            Json(BoolCodeResponse::error(e.to_string()))
        }
    }
}

/// GET /api/user/query - Get users by department ID
pub async fn get_users_by_dept(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<DepartmentIdQuery>,
) -> Json<ApiResponse<Vec<UserResponse>>> {
    let dept_name = get_department_name(&*db, query.department_id).await;
    match user::Entity::find()
        .filter(user::Column::DepartmentId.eq(query.department_id))
        .order_by_asc(user::Column::Id)
        .all(&*db)
        .await
    {
        Ok(users) => {
            // Fetch roles from Casbin for each user
            let perm_enforcer = state.get_perm().await;
            let mut response: Vec<UserResponse> = Vec::new();

            for u in users {
                let (role, direct_permissions) = if let Some(ref enforcer) = perm_enforcer {
                    let role = enforcer.get_user_role(&u.username).await.ok().flatten();
                    let perms = enforcer.get_direct_permissions(&u.username).await.unwrap_or_default();
                    (role, perms)
                } else {
                    (None, Vec::new())
                };
                let effective_quota = get_effective_quota(&*db, u.department_id, u.quota.clone()).await;
                response.push(UserResponse::from_model_with_role(u, role, direct_permissions, effective_quota));
            }

            // Log operation
            let op_desc = format!("所属部门: {}", dept_name);
            log_operation(&current_user.username, OP_QUERY_USER, &op_desc, OP_SUCCESS, None);
            Json(ApiResponse::success(response))
        }
        Err(e) => {
            tracing::error!("Failed to get users: {}", e);
            Json(ApiResponse::error(500, e.to_string()))
        }
    }
}

/// GET /api/user/info - Get user by username
pub async fn get_user_by_username(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Query(query): Query<UsernameQuery>,
) -> Json<ApiResponse<Option<UserResponse>>> {
    match user::Entity::find()
        .filter(user::Column::Username.eq(&query.username))
        .one(&*db)
        .await
    {
        Ok(Some(u)) => {
            // Fetch role from Casbin
            let perm_enforcer = state.get_perm().await;
            let (role, direct_permissions) = if let Some(ref enforcer) = perm_enforcer {
                let role = enforcer.get_user_role(&u.username).await.ok().flatten();
                let perms = enforcer.get_direct_permissions(&u.username).await.unwrap_or_default();
                (role, perms)
            } else {
                (None, Vec::new())
            };
            let effective_quota = get_effective_quota(&*db, u.department_id, u.quota.clone()).await;
            Json(ApiResponse::success(Some(UserResponse::from_model_with_role(
                u,
                role,
                direct_permissions,
                effective_quota,
            ))))
        }
        Ok(None) => Json(ApiResponse::error(404, "用户不存在")),
        Err(e) => {
            tracing::error!("Failed to get user: {}", e);
            Json(ApiResponse::error(500, e.to_string()))
        }
    }
}

/// POST /api/user/enable
pub async fn enable_user(
    State(_state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(users): Json<Vec<UserStatusItem>>,
) -> Json<BoolCodeResponse> {
    // Permission check: only admin can enable users
    if !can_manage_users(&current_user) {
        return Json(BoolCodeResponse::error("权限不足，仅管理员可启用用户"));
    }

    let mut success_count = 0;
    let mut error_count = 0;

    for u in users {
        let update = user::ActiveModel {
            id: Set(u.id),
            status: Set(1),
            ..Default::default()
        };

        let op_desc = format!("用户名: {}", u.username);
        match update.update(&*db).await {
            Ok(_) => {
                success_count += 1;
                log_operation(&current_user.username, OP_ENABLE_USER, &op_desc, OP_SUCCESS, None);
            }
            Err(e) => {
                tracing::error!("Failed to enable user {}: {}", u.username, e);
                error_count += 1;
                log_operation(&current_user.username, OP_ENABLE_USER, &op_desc, OP_FAILED, None);
            }
        }
    }

    let message = format!("成功启用{}个用户, 失败{}个", success_count, error_count);
    Json(BoolCodeResponse::success(message))
}

/// POST /api/user/disable
pub async fn disable_user(
    State(_state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(users): Json<Vec<UserStatusItem>>,
) -> Json<BoolCodeResponse> {
    // Permission check: only admin can disable users
    if !can_manage_users(&current_user) {
        return Json(BoolCodeResponse::error("权限不足，仅管理员可禁用用户"));
    }

    let mut success_count = 0;
    let mut error_count = 0;

    for u in users {
        let update = user::ActiveModel {
            id: Set(u.id),
            status: Set(2),
            ..Default::default()
        };

        let op_desc = format!("用户名: {}", u.username);
        match update.update(&*db).await {
            Ok(_) => {
                success_count += 1;
                log_operation(&current_user.username, OP_DISABLE_USER, &op_desc, OP_SUCCESS, None);
            }
            Err(e) => {
                tracing::error!("Failed to disable user {}: {}", u.username, e);
                error_count += 1;
                log_operation(&current_user.username, OP_DISABLE_USER, &op_desc, OP_FAILED, None);
            }
        }
    }

    let message = format!("成功禁用{}个用户, 失败{}个", success_count, error_count);
    Json(BoolCodeResponse::success(message))
}

/// POST /api/user/change-password
pub async fn change_password(
    State(_state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<ChangePasswordRequest>,
) -> Json<ApiResponse<()>> {
    let db_user = match user::Entity::find()
        .filter(user::Column::Username.eq(&current_user.username))
        .one(&*db)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => return Json(ApiResponse::error(1, "用户不存在")),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(1, "internal error"));
        }
    };

    if !bcrypt::verify(&req.old_password, &db_user.password).unwrap_or(false) {
        return Json(ApiResponse::error(1, "原密码错误"));
    }

    let new_hash = match bcrypt::hash(&req.new_password, 12) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash password: {}", e);
            return Json(ApiResponse::error(1, "密码加密失败"));
        }
    };

    let update = user::ActiveModel {
        id: Set(db_user.id),
        password: Set(new_hash),
        ..Default::default()
    };

    match update.update(&*db).await {
        Ok(_) => {
            // Log operation
            log_operation(&current_user.username, OP_UPDATE_PASSWORD, "修改密码", OP_SUCCESS, None);
            Json(ApiResponse::success_msg("success"))
        }
        Err(e) => {
            tracing::error!("Failed to update password: {}", e);
            Json(ApiResponse::error(1, "更新密码失败"))
        }
    }
}

/// POST /api/user/reset-password - Admin resets user password
pub async fn reset_password(
    State(_state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<ResetPasswordRequest>,
) -> Json<BoolCodeResponse> {
    // Hash the new password
    let new_hash = match bcrypt::hash(&req.password, 12) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash password: {}", e);
            return Json(BoolCodeResponse::error("密码加密失败"));
        }
    };

    let update = user::ActiveModel {
        id: Set(req.id),
        password: Set(new_hash),
        ..Default::default()
    };

    match update.update(&*db).await {
        Ok(_) => {
            // Log operation
            let op_desc = format!("用户名: {}", req.username);
            log_operation(&current_user.username, OP_UPDATE_PASSWORD, &op_desc, OP_SUCCESS, None);
            Json(BoolCodeResponse::success("密码修改成功"))
        }
        Err(e) => {
            tracing::error!("Failed to reset password: {}", e);
            Json(BoolCodeResponse::error("重置密码失败"))
        }
    }
}

/// Helper function to get department names (full path like Go version)
fn get_department_names(db: &sea_orm::DatabaseConnection, id: i64) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send + '_>> {
    use crate::entity::department;

    Box::pin(async move {
        match department::Entity::find_by_id(id).one(db).await {
            Ok(Some(d)) => {
                if d.parent_id != 0 {
                    let parent_names = get_department_names(db, d.parent_id).await;
                    if parent_names.is_empty() {
                        d.name
                    } else {
                        format!("{}/{}", parent_names, d.name)
                    }
                } else {
                    d.name
                }
            }
            _ => String::new(),
        }
    })
}

/// Helper function to get department name (wrapper for easier use)
async fn get_department_name(db: &sea_orm::DatabaseConnection, id: i64) -> String {
    get_department_names(db, id).await
}

fn normalize_quota(quota: Option<String>) -> Option<String> {
    quota.and_then(|q| {
        let trimmed = q.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// Resolve effective quota (user overrides department, department inherits parent)
async fn get_effective_quota(
    db: &sea_orm::DatabaseConnection,
    department_id: i64,
    user_quota: Option<String>,
) -> Option<String> {
    if user_quota.is_some() {
        return user_quota;
    }

    use crate::entity::department;
    let mut current_id = department_id;

    while current_id != 0 {
        match department::Entity::find_by_id(current_id).one(db).await {
            Ok(Some(dept)) => {
                if dept.quota.is_some() {
                    return dept.quota;
                }
                current_id = dept.parent_id;
            }
            _ => break,
        }
    }

    None
}

/// GET /api/user/avatar/:username - Get user avatar
pub async fn get_user_avatar(
    State(state): State<AppState>,
    Extension(_db): Extension<DbConn>,
    Path(username): Path<String>,
) -> impl IntoResponse {
    let avatar_path = state.config.root_dir.join("avatar").join(&username).join("avatar.png");

    // Check if avatar exists
    if !avatar_path.exists() {
        // Create default avatar
        if let Err(e) = create_default_avatar(&state.config.root_dir, &username).await {
            tracing::error!("Failed to create default avatar: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(r#"{"error": "internal error"}"#),
            ).into_response();
        }
    }

    // Read avatar file
    match tokio::fs::read(&avatar_path).await {
        Ok(data) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/png")
                .header(header::CACHE_CONTROL, "public, max-age=86400")
                .body(Body::from(data))
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to read avatar: {}", e);
            (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(r#"{"error": "avatar not found"}"#),
            ).into_response()
        }
    }
}

/// POST /api/user/upload/avatar - Upload user avatar
pub async fn upload_user_avatar(
    State(state): State<AppState>,
    Extension(_db): Extension<DbConn>,
    mut multipart: Multipart,
) -> Json<ApiResponse<serde_json::Value>> {
    let mut username = String::new();
    let mut avatar_data: Option<Vec<u8>> = None;

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.ok().flatten() {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "username" => {
                if let Ok(text) = field.text().await {
                    username = text;
                }
            }
            "avatar" => {
                if let Ok(bytes) = field.bytes().await {
                    avatar_data = Some(bytes.to_vec());
                }
            }
            _ => {}
        }
    }

    if username.is_empty() {
        return Json(ApiResponse::error(400, "用户名不能为空"));
    }

    let avatar_data = match avatar_data {
        Some(d) => d,
        None => return Json(ApiResponse::error(400, "上传头像文件错误")),
    };

    // Create avatar directory
    let avatar_dir = state.config.root_dir.join("avatar").join(&username);
    if let Err(e) = tokio::fs::create_dir_all(&avatar_dir).await {
        tracing::error!("Failed to create avatar directory: {}", e);
        return Json(ApiResponse::error(500, "创建头像目录失败"));
    }

    // Save avatar file
    let avatar_path = avatar_dir.join("avatar.png");
    if let Err(e) = tokio::fs::write(&avatar_path, &avatar_data).await {
        tracing::error!("Failed to save avatar: {}", e);
        return Json(ApiResponse::error(500, "保存头像失败"));
    }

    Json(ApiResponse::success(serde_json::json!({
        "large": format!("/api/user/avatar/{}", username)
    })))
}

/// DELETE /api/user/avatar/:username - Delete user avatar
pub async fn delete_user_avatar(
    State(state): State<AppState>,
    Extension(_db): Extension<DbConn>,
    Path(username): Path<String>,
) -> Json<ApiResponse<()>> {
    if username.is_empty() {
        return Json(ApiResponse::error(400, "用户名不能为空"));
    }

    let avatar_path = state.config.root_dir.join("avatar").join(&username).join("avatar.png");

    // Delete avatar file
    if avatar_path.exists() {
        if let Err(e) = tokio::fs::remove_file(&avatar_path).await {
            tracing::error!("Failed to delete avatar: {}", e);
            return Json(ApiResponse::error(500, "删除头像失败"));
        }
    }

    Json(ApiResponse::success_msg("success"))
}

/// Create a default avatar with random color
async fn create_default_avatar(root_dir: &std::path::Path, username: &str) -> std::io::Result<()> {
    let avatar_dir = root_dir.join("avatar").join(username);
    let avatar_path = avatar_dir.join("avatar.png");

    // Create directory
    tokio::fs::create_dir_all(&avatar_dir).await?;

    // Generate random color
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    // Simple random number generator
    let r = ((seed >> 16) & 0xFF) as u8;
    let g = ((seed >> 8) & 0xFF) as u8;
    let b = (seed & 0xFF) as u8;

    // Create a simple 150x150 PNG with solid color
    // Using a minimal PNG structure
    let png_data = create_solid_color_png(150, 150, r, g, b);

    tokio::fs::write(&avatar_path, &png_data).await?;
    Ok(())
}

/// Create a minimal PNG with solid color
fn create_solid_color_png(width: u32, height: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
    use std::io::Write;

    // PNG signature
    let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    // IHDR chunk
    let mut ihdr = Vec::new();
    ihdr.write_all(&width.to_be_bytes()).unwrap();
    ihdr.write_all(&height.to_be_bytes()).unwrap();
    ihdr.push(8);  // bit depth
    ihdr.push(2);  // color type (RGB)
    ihdr.push(0);  // compression
    ihdr.push(0);  // filter
    ihdr.push(0);  // interlace

    write_png_chunk(&mut data, b"IHDR", &ihdr);

    // IDAT chunk (image data)
    let mut raw_data = Vec::new();
    for _ in 0..height {
        raw_data.push(0); // filter byte
        for _ in 0..width {
            raw_data.push(r);
            raw_data.push(g);
            raw_data.push(b);
        }
    }

    // Compress with deflate
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&raw_data, 6);
    write_png_chunk(&mut data, b"IDAT", &compressed);

    // IEND chunk
    write_png_chunk(&mut data, b"IEND", &[]);

    data
}

/// Write a PNG chunk
fn write_png_chunk(data: &mut Vec<u8>, chunk_type: &[u8; 4], chunk_data: &[u8]) {
    use std::io::Write;

    // Length
    data.write_all(&(chunk_data.len() as u32).to_be_bytes()).unwrap();

    // Type
    data.write_all(chunk_type).unwrap();

    // Data
    data.write_all(chunk_data).unwrap();

    // CRC32
    let mut crc_data = chunk_type.to_vec();
    crc_data.extend_from_slice(chunk_data);
    let crc = crc32fast::hash(&crc_data);
    data.write_all(&crc.to_be_bytes()).unwrap();
}
