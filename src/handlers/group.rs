//! Group handlers
//!
//! Implements group CRUD and member management operations

use axum::{
    extract::Query,
    response::Json,
    Extension,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::entity::{group, group_user, user};
use crate::handlers::audit::service::log_operation;
use crate::middleware::auth::CurrentUser;
use crate::middleware::DbConn;
use crate::routes::ApiResponse;

// Operation types (matching Go version)
const OP_CREATE_GROUP: &str = "添加群组";
const OP_DELETE_GROUP: &str = "删除群组";
const OP_QUERY_GROUP: &str = "查询群组";
const OP_ADD_GROUP_USER: &str = "添加群组用户";
const OP_DEL_GROUP_USER: &str = "删除群组用户";
const OP_QUERY_GROUP_USER: &str = "查询群组用户";
const OP_SUCCESS: &str = "成功";

/// Add group request
#[derive(Debug, Deserialize)]
pub struct AddGroupRequest {
    pub name: String,
}

/// Group response
#[derive(Debug, Serialize)]
pub struct GroupResponse {
    pub id: i64,
    pub name: String,
    pub owner: bool,
}

/// Group user response
#[derive(Debug, Serialize)]
pub struct GroupUserResponse {
    pub id: i64,
    pub username: String,
    #[serde(rename = "fullName")]
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}

/// Query parameters
#[derive(Debug, Deserialize)]
pub struct IdQuery {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct GroupIdQuery {
    #[serde(rename = "groupId")]
    pub group_id: i64,
}

/// POST /api/group/add
pub async fn add_group(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<AddGroupRequest>,
) -> Json<ApiResponse<Option<GroupResponse>>> {
    // Check if group name already exists
    let existing = group::Entity::find()
        .filter(group::Column::Name.eq(&req.name))
        .one(&*db)
        .await;

    match existing {
        Ok(Some(_)) => return Json(ApiResponse::error(400, "组名称已存在")),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, "internal error"));
        }
        Ok(None) => {}
    }

    // Create group in transaction
    let result = (&*db).transaction::<_, group::Model, sea_orm::DbErr>(|txn| {
        Box::pin(async move {
            // Create group
            let new_group = group::ActiveModel {
                name: Set(req.name.clone()),
                ..Default::default()
            };
            let group = new_group.insert(txn).await?;

            // Add current user as owner
            let group_user = group_user::ActiveModel {
                group_id: Set(group.id),
                user_id: Set(current_user.id),
                owner: Set(true),
                ..Default::default()
            };
            group_user.insert(txn).await?;

            Ok(group)
        })
    }).await;

    match result {
        Ok(group) => {
            // Log operation
            let op_desc = format!("群组名称: {}", group.name);
            log_operation(&current_user.username, OP_CREATE_GROUP, &op_desc, OP_SUCCESS, None);
            Json(ApiResponse::success(Some(GroupResponse {
                id: group.id,
                name: group.name,
                owner: true,
            })))
        }
        Err(e) => {
            tracing::error!("Failed to create group: {}", e);
            Json(ApiResponse::error(500, e.to_string()))
        }
    }
}

/// POST /api/group/delete
pub async fn delete_group(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<IdQuery>,
) -> Json<ApiResponse<()>> {
    // Check if group exists
    let group_result = group::Entity::find_by_id(query.id)
        .one(&*db)
        .await;

    let group_info = match group_result {
        Ok(Some(g)) => g,
        Ok(None) => return Json(ApiResponse::error(400, "未找到该群组")),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, "internal error"));
        }
    };

    // Delete group and members in transaction
    let result = (&*db).transaction::<_, (), sea_orm::DbErr>(|txn| {
        Box::pin(async move {
            // Delete group members
            group_user::Entity::delete_many()
                .filter(group_user::Column::GroupId.eq(query.id))
                .exec(txn)
                .await?;

            // Delete group
            group::Entity::delete_by_id(query.id)
                .exec(txn)
                .await?;

            Ok(())
        })
    }).await;

    match result {
        Ok(_) => {
            // Log operation
            let op_desc = format!("群组名称: {}", group_info.name);
            log_operation(&current_user.username, OP_DELETE_GROUP, &op_desc, OP_SUCCESS, None);
            Json(ApiResponse::success_msg("success"))
        }
        Err(e) => {
            tracing::error!("Failed to delete group: {}", e);
            Json(ApiResponse::error(500, e.to_string()))
        }
    }
}

/// GET /api/group/query - Get groups for current user
pub async fn get_groups(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
) -> Json<ApiResponse<Vec<GroupResponse>>> {
    // Get groups where user is a member
    let group_users = group_user::Entity::find()
        .filter(group_user::Column::UserId.eq(current_user.id))
        .all(&*db)
        .await;

    let group_users = match group_users {
        Ok(gu) => gu,
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, e.to_string()));
        }
    };

    let mut groups = Vec::new();
    for gu in group_users {
        let group = group::Entity::find_by_id(gu.group_id)
            .one(&*db)
            .await;

        if let Ok(Some(g)) = group {
            groups.push(GroupResponse {
                id: g.id,
                name: g.name,
                owner: gu.owner,
            });
        }
    }

    // Log operation
    log_operation(&current_user.username, OP_QUERY_GROUP, "", OP_SUCCESS, None);
    Json(ApiResponse::success(groups))
}

/// POST /api/group/addUsers
pub async fn add_users_to_group(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<GroupIdQuery>,
    Json(user_ids): Json<Vec<i64>>,
) -> Json<ApiResponse<()>> {
    // Check if group exists
    let group_result = group::Entity::find_by_id(query.group_id)
        .one(&*db)
        .await;

    let group_info = match group_result {
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, "internal error"));
        }
        Ok(None) => {
            return Json(ApiResponse::error(400, "未找到该群组"));
        }
        Ok(Some(g)) => g,
    };

    // Add users to group
    let result = (&*db).transaction::<_, (), sea_orm::DbErr>(|txn| {
        Box::pin(async move {
            for user_id in user_ids {
                // Check if user exists
                let user_exists = user::Entity::find_by_id(user_id)
                    .one(txn)
                    .await?;

                if user_exists.is_none() {
                    continue;
                }

                // Check if user is already in group
                let existing = group_user::Entity::find()
                    .filter(group_user::Column::GroupId.eq(query.group_id))
                    .filter(group_user::Column::UserId.eq(user_id))
                    .one(txn)
                    .await?;

                if existing.is_some() {
                    continue;
                }

                // Add user to group
                let new_member = group_user::ActiveModel {
                    group_id: Set(query.group_id),
                    user_id: Set(user_id),
                    owner: Set(false),
                    ..Default::default()
                };
                new_member.insert(txn).await?;
            }
            Ok(())
        })
    }).await;

    match result {
        Ok(_) => {
            // Log operation
            let op_desc = format!("群组名称: {}", group_info.name);
            log_operation(&current_user.username, OP_ADD_GROUP_USER, &op_desc, OP_SUCCESS, None);
            Json(ApiResponse::success_msg("success"))
        }
        Err(e) => {
            tracing::error!("Failed to add users to group: {}", e);
            Json(ApiResponse::error(500, e.to_string()))
        }
    }
}

/// POST /api/group/deleteUsers
pub async fn delete_users_from_group(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<GroupIdQuery>,
    Json(user_ids): Json<Vec<i64>>,
) -> Json<ApiResponse<()>> {
    // Check if group exists
    let group_result = group::Entity::find_by_id(query.group_id)
        .one(&*db)
        .await;

    let group_info = match group_result {
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, "internal error"));
        }
        Ok(None) => {
            return Json(ApiResponse::error(400, "未找到该群组"));
        }
        Ok(Some(g)) => g,
    };

    // Delete users from group
    let result = (&*db).transaction::<_, (), sea_orm::DbErr>(|txn| {
        Box::pin(async move {
            for user_id in user_ids {
                group_user::Entity::delete_many()
                    .filter(group_user::Column::GroupId.eq(query.group_id))
                    .filter(group_user::Column::UserId.eq(user_id))
                    .exec(txn)
                    .await?;
            }
            Ok(())
        })
    }).await;

    match result {
        Ok(_) => {
            // Log operation
            let op_desc = format!("群组名称: {}", group_info.name);
            log_operation(&current_user.username, OP_DEL_GROUP_USER, &op_desc, OP_SUCCESS, None);
            Json(ApiResponse::success_msg("success"))
        }
        Err(e) => {
            tracing::error!("Failed to delete users from group: {}", e);
            Json(ApiResponse::error(500, e.to_string()))
        }
    }
}

/// GET /api/group/query/users - Get group members
pub async fn get_group_users(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<GroupIdQuery>,
) -> Json<ApiResponse<Vec<GroupUserResponse>>> {
    // Check if group exists
    let group_result = group::Entity::find_by_id(query.group_id)
        .one(&*db)
        .await;

    match group_result {
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, e.to_string()));
        }
        Ok(None) => {
            return Json(ApiResponse::error(400, "未找到该群组"));
        }
        Ok(Some(_)) => {}
    }

    // Get group members (excluding current user)
    let group_users = group_user::Entity::find()
        .filter(group_user::Column::GroupId.eq(query.group_id))
        .filter(group_user::Column::UserId.ne(current_user.id))
        .all(&*db)
        .await;

    let group_users = match group_users {
        Ok(gu) => gu,
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Json(ApiResponse::error(500, e.to_string()));
        }
    };

    let mut users = Vec::new();
    for gu in group_users {
        let u = user::Entity::find_by_id(gu.user_id)
            .one(&*db)
            .await;

        if let Ok(Some(u)) = u {
            users.push(GroupUserResponse {
                id: u.id,
                username: u.username,
                full_name: u.full_name,
                email: u.email,
                phone: u.phone,
            });
        }
    }

    // Log operation
    log_operation(&current_user.username, OP_QUERY_GROUP_USER, "", OP_SUCCESS, None);
    Json(ApiResponse::success(users))
}
