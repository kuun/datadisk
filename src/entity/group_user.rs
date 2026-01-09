//! GroupUser entity - 群组成员关系表
//!
//! 对应 Go 模型: models/group.go (GroupUser struct)
//! 表名: disk_group_user

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "disk_group_user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 用户ID
    pub user_id: i64,

    /// 群组ID
    pub group_id: i64,

    /// 是否为群组所有者
    pub owner: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

// 跨模块关系通过手动查询处理

impl ActiveModelBehavior for ActiveModel {}

/// 群组成员响应 (包含用户详情)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupMemberResponse {
    pub id: i64,
    pub user_id: i64,
    pub group_id: i64,
    pub owner: bool,
    pub username: Option<String>,
    pub full_name: Option<String>,
}

impl From<Model> for GroupMemberResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            group_id: model.group_id,
            owner: model.owner,
            username: None,
            full_name: None,
        }
    }
}

impl GroupMemberResponse {
    pub fn with_user_info(mut self, username: String, full_name: String) -> Self {
        self.username = Some(username);
        self.full_name = Some(full_name);
        self
    }
}
