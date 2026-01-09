//! FileInfo entity - 文件信息表
//!
//! 对应 Go 模型: models/fileinfo.go
//! 表名: disk_file_info

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "disk_file_info")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 父目录ID (-1 表示根目录)
    pub parent_id: i64,

    /// 父路径
    #[sea_orm(column_type = "String(Some(512))", nullable)]
    pub parent_path: Option<String>,

    /// 所有者用户名
    #[sea_orm(column_type = "String(Some(32))")]
    pub username: String,

    /// 文件/目录名称
    #[sea_orm(column_type = "String(Some(256))")]
    pub name: String,

    /// 文件类型 (MIME 类型或 "dir")
    #[sea_orm(column_name = "type", column_type = "String(Some(64))")]
    pub file_type: String,

    /// 文件大小 (字节)
    pub size: i64,

    /// 创建时间 (Unix 时间戳)
    pub create_time: i64,

    /// 修改时间 (Unix 时间戳)
    pub modify_time: i64,

    /// 是否为目录
    pub is_directory: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

// 自引用和跨模块关系通过手动查询处理

impl ActiveModelBehavior for ActiveModel {}

/// 文件列表响应项
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileListItem {
    pub id: i64,
    pub basename: String,
    pub name: String,
    #[serde(rename = "type")]
    pub file_type: String,
    pub mime: String,
    pub size: i64,
    pub create_time: i64,
    pub modify_time: i64,
    pub is_directory: bool,
}

impl From<Model> for FileListItem {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            basename: model.name.clone(),
            name: model.name,
            file_type: if model.is_directory {
                "directory".to_string()
            } else {
                "file".to_string()
            },
            mime: model.file_type,
            size: model.size,
            create_time: model.create_time,
            modify_time: model.modify_time,
            is_directory: model.is_directory,
        }
    }
}
