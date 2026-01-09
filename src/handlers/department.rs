//! Department handlers
//!
//! Implements department CRUD operations

use axum::{
    extract::Query,
    response::Json,
    Extension,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};

use crate::entity::department;
use crate::handlers::audit::service::log_operation;
use crate::middleware::auth::CurrentUser;
use crate::middleware::DbConn;
use crate::routes::ApiResponse;

// Operation types (matching Go version)
const OP_CREATE_DEPT: &str = "创建部门信息";
const OP_DELETE_DEPT: &str = "删除部门信息";
const OP_UPDATE_DEPT: &str = "修改部门信息";
const OP_QUERY_DEPT: &str = "查询部门信息";
const OP_SUCCESS: &str = "成功";

/// Check if user has contacts permission (for department management)
fn can_manage_departments(user: &CurrentUser) -> bool {
    user.can_contacts()
}

/// Add department request
#[derive(Debug, Deserialize)]
pub struct AddDepartmentRequest {
    pub name: String,
    pub level: Option<i32>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
}

/// Update department request
#[derive(Debug, Deserialize)]
pub struct UpdateDepartmentRequest {
    pub id: i64,
    pub name: String,
    pub level: Option<i32>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
    #[serde(rename = "parentName")]
    pub parent_name: Option<String>,
}

/// Department response
#[derive(Debug, Serialize)]
pub struct DepartmentResponse {
    pub id: i64,
    pub name: String,
    pub level: i32,
    #[serde(rename = "parentId")]
    pub parent_id: i64,
    #[serde(rename = "parentName")]
    pub parent_name: String,
}

impl From<department::Model> for DepartmentResponse {
    fn from(m: department::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            level: m.level,
            parent_id: m.parent_id,
            parent_name: m.parent_name,
        }
    }
}

/// Query parameters for delete
#[derive(Debug, Deserialize)]
pub struct IdQuery {
    pub id: i64,
}

/// POST /api/departments/add
pub async fn add_department(
    Extension(db): Extension<DbConn>,
    Extension(user): Extension<CurrentUser>,
    Json(req): Json<AddDepartmentRequest>,
) -> Json<ApiResponse<Option<DepartmentResponse>>> {
    // Permission check: only admin can add departments
    if !can_manage_departments(&user) {
        return Json(ApiResponse::error(403, "权限不足，仅管理员可创建部门"));
    }

    if req.name.chars().count() > 32 {
        return Json(ApiResponse::error(400, "部门名称不能超过32个字符"));
    }

    let parent_id = req.parent_id.unwrap_or(0);

    let existing = department::Entity::find()
        .filter(department::Column::Name.eq(&req.name))
        .filter(department::Column::ParentId.eq(parent_id))
        .one(&*db)
        .await;

    match existing {
        Ok(Some(_)) => return Json(ApiResponse::error(0, "部门名称已存在")),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, "internal error"));
        }
        Ok(None) => {}
    }

    let parent_name = if parent_id > 0 {
        get_department_path(&*db, parent_id).await
    } else {
        String::new()
    };

    let new_dept = department::ActiveModel {
        name: Set(req.name.clone()),
        level: Set(req.level.unwrap_or(1)),
        parent_id: Set(parent_id),
        parent_name: Set(parent_name.clone()),
        ..Default::default()
    };

    match new_dept.insert(&*db).await {
        Ok(dept) => {
            // Log operation
            let op_desc = if parent_name.is_empty() {
                format!("部门名称: {}", req.name)
            } else {
                format!("部门名称: {}/{}", parent_name, req.name)
            };
            log_operation(&user.username, OP_CREATE_DEPT, &op_desc, OP_SUCCESS, None);
            Json(ApiResponse::success(Some(DepartmentResponse::from(dept))))
        }
        Err(e) => {
            tracing::error!("Failed to create department: {}", e);
            Json(ApiResponse::error(500, e.to_string()))
        }
    }
}

/// POST /api/department/delete
pub async fn delete_department(
    Extension(db): Extension<DbConn>,
    Extension(user): Extension<CurrentUser>,
    Query(query): Query<IdQuery>,
) -> Json<ApiResponse<()>> {
    // Permission check: only admin can delete departments
    if !can_manage_departments(&user) {
        return Json(ApiResponse::error(403, "权限不足，仅管理员可删除部门"));
    }

    let has_children = department::Entity::find()
        .filter(department::Column::ParentId.eq(query.id))
        .one(&*db)
        .await;

    match has_children {
        Ok(Some(_)) => return Json(ApiResponse::error(0, "子部门不为空，不能删除")),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, "internal error"));
        }
        Ok(None) => {}
    }

    let dept = department::Entity::find_by_id(query.id)
        .one(&*db)
        .await;

    let dept_info = match dept {
        Ok(Some(d)) => d,
        Ok(None) => return Json(ApiResponse::error(0, "部门不存在")),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, "internal error"));
        }
    };

    match department::Entity::delete_by_id(query.id).exec(&*db).await {
        Ok(_) => {
            // Log operation
            let op_desc = if dept_info.parent_name.is_empty() {
                format!("部门名称: {}", dept_info.name)
            } else {
                format!("部门名称: {}/{}", dept_info.parent_name, dept_info.name)
            };
            log_operation(&user.username, OP_DELETE_DEPT, &op_desc, OP_SUCCESS, None);
            Json(ApiResponse::success_msg("success"))
        }
        Err(e) => {
            tracing::error!("Failed to delete department: {}", e);
            Json(ApiResponse::error(500, "删除失败"))
        }
    }
}

/// POST /api/department/update
pub async fn update_department(
    Extension(db): Extension<DbConn>,
    Extension(user): Extension<CurrentUser>,
    Json(req): Json<UpdateDepartmentRequest>,
) -> Json<ApiResponse<Option<DepartmentResponse>>> {
    // Permission check: only admin can update departments
    if !can_manage_departments(&user) {
        return Json(ApiResponse::error(403, "权限不足，仅管理员可修改部门"));
    }

    if req.name.chars().count() > 32 {
        return Json(ApiResponse::error(400, "部门名称不能超过32个字符"));
    }

    let parent_id = req.parent_id.unwrap_or(0);

    let existing = department::Entity::find()
        .filter(department::Column::Name.eq(&req.name))
        .filter(department::Column::ParentId.eq(parent_id))
        .filter(department::Column::Id.ne(req.id))
        .one(&*db)
        .await;

    match existing {
        Ok(Some(_)) => return Json(ApiResponse::error(400, "部门名称已存在")),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, "internal error"));
        }
        Ok(None) => {}
    }

    let parent_name = req.parent_name.clone().unwrap_or_default();
    let update_model = department::ActiveModel {
        id: Set(req.id),
        name: Set(req.name.clone()),
        level: Set(req.level.unwrap_or(1)),
        parent_id: Set(parent_id),
        parent_name: Set(parent_name.clone()),
    };

    match update_model.update(&*db).await {
        Ok(dept) => {
            // Log operation
            let op_desc = if parent_name.is_empty() {
                format!("部门名称: {}", req.name)
            } else {
                format!("部门名称: {}/{}", parent_name, req.name)
            };
            log_operation(&user.username, OP_UPDATE_DEPT, &op_desc, OP_SUCCESS, None);
            Json(ApiResponse::success(Some(DepartmentResponse::from(dept))))
        }
        Err(e) => {
            tracing::error!("Failed to update department: {}", e);
            Json(ApiResponse::error(500, e.to_string()))
        }
    }
}

/// Response format matching Go version: {"success": true, "data": [...]}
#[derive(Debug, Serialize)]
pub struct DeptQueryResponse {
    pub success: bool,
    pub data: Vec<DepartmentResponse>,
}

/// GET /api/department/query
pub async fn get_departments(
    Extension(db): Extension<DbConn>,
    Extension(user): Extension<CurrentUser>,
) -> Json<DeptQueryResponse> {
    match department::Entity::find()
        .order_by_asc(department::Column::Id)
        .all(&*db)
        .await
    {
        Ok(depts) => {
            let response: Vec<DepartmentResponse> = depts.into_iter().map(|d| d.into()).collect();
            // Log operation
            log_operation(&user.username, OP_QUERY_DEPT, "", OP_SUCCESS, None);
            Json(DeptQueryResponse {
                success: true,
                data: response,
            })
        }
        Err(e) => {
            tracing::error!("Failed to get departments: {}", e);
            Json(DeptQueryResponse {
                success: false,
                data: vec![],
            })
        }
    }
}

/// GET /api/department/query/all - Get departments and users tree
#[derive(Debug, Serialize)]
pub struct DeptUserTreeItem {
    pub id_: String,
    pub id: i64,
    pub name: String,
    #[serde(rename = "parentId")]
    pub parent_id: i64,
    #[serde(rename = "parentId_")]
    pub parent_id_: String,
    #[serde(rename = "isDept")]
    pub is_dept: bool,
}

/// Response format matching Go version: {"success": true, "data": [...]}
#[derive(Debug, Serialize)]
pub struct DeptUsersResponse {
    pub success: bool,
    pub data: Vec<DeptUserTreeItem>,
}

pub async fn get_dept_and_users(
    Extension(db): Extension<DbConn>,
    Extension(_user): Extension<CurrentUser>,
) -> Json<DeptUsersResponse> {
    use crate::entity::user;

    let departments = match department::Entity::find().all(&*db).await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to get departments: {}", e);
            return Json(DeptUsersResponse {
                success: false,
                data: vec![],
            });
        }
    };

    let mut data: Vec<DeptUserTreeItem> = Vec::new();

    for dept in departments {
        data.push(DeptUserTreeItem {
            id_: format!("dept_{}", dept.id),
            id: dept.id,
            name: dept.name,
            parent_id: dept.parent_id,
            parent_id_: format!("dept_{}", dept.parent_id),
            is_dept: true,
        });
    }

    match user::Entity::find()
        .filter(user::Column::Username.ne("admin"))
        .all(&*db)
        .await
    {
        Ok(users) => {
            for u in users {
                data.push(DeptUserTreeItem {
                    id_: format!("user_{}", u.id),
                    id: u.id,
                    name: u.username,
                    parent_id: u.department_id,
                    parent_id_: format!("dept_{}", u.department_id),
                    is_dept: false,
                });
            }
            Json(DeptUsersResponse {
                success: true,
                data,
            })
        }
        Err(e) => {
            tracing::error!("Failed to get users: {}", e);
            Json(DeptUsersResponse {
                success: false,
                data: vec![],
            })
        }
    }
}

/// Helper function to get department path (parent names)
async fn get_department_path(db: &sea_orm::DatabaseConnection, id: i64) -> String {
    let dept = department::Entity::find_by_id(id).one(db).await;

    match dept {
        Ok(Some(d)) => {
            if d.parent_id != 0 {
                let parent_path = Box::pin(get_department_path(db, d.parent_id)).await;
                if parent_path.is_empty() {
                    d.name
                } else {
                    format!("{}/{}", parent_path, d.name)
                }
            } else {
                d.name
            }
        }
        _ => String::new(),
    }
}
