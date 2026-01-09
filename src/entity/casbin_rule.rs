//! CasbinRule entity - Casbin 策略表
//!
//! 存储 Casbin RBAC 权限策略
//! 表名: disk_casbin_rule

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "disk_casbin_rule")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 策略类型: 'p' (policy) 或 'g' (grouping/role)
    #[sea_orm(column_type = "String(Some(10))")]
    pub ptype: String,

    /// v0: 对于 'p' 是 subject(角色), 对于 'g' 是 user
    #[sea_orm(column_type = "String(Some(64))")]
    pub v0: String,

    /// v1: 对于 'p' 是 object(资源), 对于 'g' 是 role
    #[sea_orm(column_type = "String(Some(64))")]
    pub v1: String,

    /// v2: 对于 'p' 是 action(操作), 对于 'g' 通常为空
    #[sea_orm(column_type = "String(Some(64))", nullable)]
    pub v2: Option<String>,

    /// v3-v5: 扩展字段，用于更复杂的策略
    #[sea_orm(column_type = "String(Some(64))", nullable)]
    pub v3: Option<String>,

    #[sea_orm(column_type = "String(Some(64))", nullable)]
    pub v4: Option<String>,

    #[sea_orm(column_type = "String(Some(64))", nullable)]
    pub v5: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// 转换为 Casbin 策略向量
    pub fn to_policy_vec(&self) -> Vec<String> {
        let mut policy = vec![self.v0.clone(), self.v1.clone()];
        if let Some(ref v2) = self.v2 {
            if !v2.is_empty() {
                policy.push(v2.clone());
            }
        }
        if let Some(ref v3) = self.v3 {
            if !v3.is_empty() {
                policy.push(v3.clone());
            }
        }
        if let Some(ref v4) = self.v4 {
            if !v4.is_empty() {
                policy.push(v4.clone());
            }
        }
        if let Some(ref v5) = self.v5 {
            if !v5.is_empty() {
                policy.push(v5.clone());
            }
        }
        policy
    }
}

/// 创建策略记录的辅助函数
pub fn new_policy(sub: &str, obj: &str, act: &str) -> ActiveModel {
    use sea_orm::Set;
    ActiveModel {
        ptype: Set("p".to_string()),
        v0: Set(sub.to_string()),
        v1: Set(obj.to_string()),
        v2: Set(Some(act.to_string())),
        ..Default::default()
    }
}

/// 创建角色分配记录的辅助函数
pub fn new_grouping(user: &str, role: &str) -> ActiveModel {
    use sea_orm::Set;
    ActiveModel {
        ptype: Set("g".to_string()),
        v0: Set(user.to_string()),
        v1: Set(role.to_string()),
        v2: Set(None),
        ..Default::default()
    }
}
