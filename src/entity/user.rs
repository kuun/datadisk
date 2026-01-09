//! User entity - 用户表
//!
//! 对应 Go 模型: models/user.go
//! 表名: disk_user

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 用户状态
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    /// 未激活
    Inactive = 0,
    /// 正常
    Active = 1,
    /// 禁用
    Disabled = 2,
}

impl From<i32> for UserStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => UserStatus::Inactive,
            1 => UserStatus::Active,
            2 => UserStatus::Disabled,
            _ => UserStatus::Inactive,
        }
    }
}

impl From<UserStatus> for i32 {
    fn from(status: UserStatus) -> Self {
        status as i32
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "disk_user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 用户名 (唯一)
    #[sea_orm(column_type = "String(Some(32))", unique)]
    pub username: String,

    /// 密码 (bcrypt 哈希)
    #[sea_orm(column_type = "String(Some(128))")]
    #[serde(skip_serializing)]
    pub password: String,

    /// 全名
    #[sea_orm(column_type = "String(Some(64))")]
    pub full_name: String,

    /// 电话
    #[sea_orm(column_type = "String(Some(20))", nullable)]
    pub phone: Option<String>,

    /// 邮箱
    #[sea_orm(column_type = "String(Some(64))", nullable)]
    pub email: Option<String>,

    /// 最后登录时间 (Unix 时间戳)
    pub last_login: i32,

    /// 部门ID
    pub department_id: i64,

    /// 部门名称 (冗余字段)
    #[sea_orm(column_type = "String(Some(64))")]
    pub dept_name: String,

    /// 用户状态: 0=未激活, 1=正常, 2=禁用
    pub status: i32,

    /// 存储配额
    #[sea_orm(column_type = "String(Some(32))", nullable)]
    pub quota: Option<String>,

    /// 用户权限 (已弃用，权限现由 Casbin 管理，保留此字段用于向后兼容)
    #[sea_orm(column_type = "String(Some(128))", default_value = "")]
    pub permissions: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

// 跨模块关系通过手动查询处理，避免循环依赖

impl ActiveModelBehavior for ActiveModel {}

/// 用户响应 (不含密码)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub last_login: i32,
    pub department_id: i64,
    pub dept_name: String,
    /// 用户角色名称 (从 Casbin 获取)
    pub role: Option<String>,
    pub status: i32,
    pub quota: Option<String>,
    pub permissions: String,
}

impl From<Model> for UserResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            full_name: model.full_name,
            phone: model.phone,
            email: model.email,
            last_login: model.last_login,
            department_id: model.department_id,
            dept_name: model.dept_name,
            role: None, // Filled in by handler from Casbin
            status: model.status,
            quota: model.quota,
            permissions: model.permissions,
        }
    }
}
