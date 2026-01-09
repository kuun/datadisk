//! FileAccess entity - 文件访问记录表 (最近文件)
//!
//! 对应 Go 模型: models/fileaccess.go
//! 表名: disk_file_access

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 访问类型
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessType {
    Download,
    Preview,
    Edit,
}

impl AccessType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessType::Download => "download",
            AccessType::Preview => "preview",
            AccessType::Edit => "edit",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "download" => AccessType::Download,
            "preview" => AccessType::Preview,
            "edit" => AccessType::Edit,
            _ => AccessType::Preview,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "disk_file_access")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 访问用户ID
    pub user_id: i64,

    /// 文件ID
    pub file_id: i64,

    /// 文件完整路径
    #[sea_orm(column_type = "String(Some(512))")]
    pub file_path: String,

    /// 文件名
    #[sea_orm(column_type = "String(Some(256))")]
    pub file_name: String,

    /// 访问时间 (Unix 时间戳)
    pub access_time: i64,

    /// 访问类型 (download, preview, edit)
    #[sea_orm(column_type = "String(Some(16))")]
    pub access_type: String,

    /// 是否为目录
    pub is_dir: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

// 跨模块关系通过手动查询处理

impl ActiveModelBehavior for ActiveModel {}

/// 最近文件响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecentFileResponse {
    pub id: i64,
    pub file_id: i64,
    pub file_path: String,
    pub file_name: String,
    pub access_time: i64,
    pub access_type: String,
    pub is_dir: bool,
}

impl From<Model> for RecentFileResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            file_id: model.file_id,
            file_path: model.file_path,
            file_name: model.file_name,
            access_time: model.access_time,
            access_type: model.access_type,
            is_dir: model.is_dir,
        }
    }
}
