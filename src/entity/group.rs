//! Group entity - 群组表
//!
//! 对应 Go 模型: models/group.go
//! 表名: disk_group

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "disk_group")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 群组名称 (最大32字符)
    #[sea_orm(column_type = "String(Some(32))", unique)]
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

// 群组成员关系通过手动查询 group_user 表处理

impl ActiveModelBehavior for ActiveModel {}

/// 群组响应 (包含用户是否为所有者)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupResponse {
    pub id: i64,
    pub name: String,
    pub owner: bool,
}

impl From<Model> for GroupResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            owner: false,
        }
    }
}

impl GroupResponse {
    pub fn with_owner(mut self, owner: bool) -> Self {
        self.owner = owner;
        self
    }
}
