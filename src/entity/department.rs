//! Department entity - 部门表
//!
//! 对应 Go 模型: models/department.go
//! 表名: disk_department

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "disk_department")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 部门名称
    #[sea_orm(column_type = "String(Some(64))")]
    pub name: String,

    /// 部门级别
    pub level: i32,

    /// 父部门ID (0 表示顶级部门)
    pub parent_id: i64,

    /// 父部门名称 (冗余字段)
    #[sea_orm(column_type = "String(Some(64))")]
    pub parent_name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

// 自引用和跨模块关系通过 Linked 或手动查询处理

impl ActiveModelBehavior for ActiveModel {}

/// 部门树节点 (用于API响应)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepartmentTree {
    pub id: i64,
    pub name: String,
    pub level: i32,
    pub parent_id: i64,
    pub parent_name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<DepartmentTree>,
}

impl From<Model> for DepartmentTree {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            level: model.level,
            parent_id: model.parent_id,
            parent_name: model.parent_name,
            children: Vec::new(),
        }
    }
}
