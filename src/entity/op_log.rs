//! OpLog entity - 操作日志表
//!
//! 对应 Go 模型: models/oplog.go
//! 表名: disk_op_log

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 操作类型
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpType {
    /// 登录
    Login,
    /// 登出
    Logout,
    /// 创建目录
    Mkdir,
    /// 访问目录/文件
    OpenFile,
    /// 删除
    Delete,
    /// 重命名
    Rename,
    /// 复制
    Copy,
    /// 移动
    Move,
    /// 上传
    Upload,
    /// 下载
    Download,
    /// 创建用户
    CreateUser,
    /// 更新用户
    UpdateUser,
    /// 删除用户
    DeleteUser,
    /// 启用用户
    EnableUser,
    /// 禁用用户
    DisableUser,
    /// 导出用户
    ExportUser,
    /// 查询用户
    QueryUser,
    /// 修改密码
    UpdatePassword,
    /// 创建部门
    CreateDept,
    /// 更新部门
    UpdateDept,
    /// 删除部门
    DeleteDept,
    /// 查询部门
    QueryDept,
    /// 创建群组
    CreateGroup,
    /// 删除群组
    DeleteGroup,
    /// 查询群组
    QueryGroup,
    /// 添加群组用户
    AddGroupUser,
    /// 删除群组用户
    DeleteGroupUser,
    /// 统计
    Stat,
    /// 更新统计
    UpdateStat,
}

impl OpType {
    /// 转换为中文显示
    pub fn to_chinese(&self) -> &'static str {
        match self {
            OpType::Login => "登录",
            OpType::Logout => "登出",
            OpType::Mkdir => "创建目录",
            OpType::OpenFile => "访问目录/文件",
            OpType::Delete => "删除",
            OpType::Rename => "重命名",
            OpType::Copy => "复制",
            OpType::Move => "移动",
            OpType::Upload => "上传",
            OpType::Download => "下载",
            OpType::CreateUser => "创建用户",
            OpType::UpdateUser => "更新用户",
            OpType::DeleteUser => "删除用户",
            OpType::EnableUser => "启用用户",
            OpType::DisableUser => "禁用用户",
            OpType::ExportUser => "导出用户",
            OpType::QueryUser => "查询用户",
            OpType::UpdatePassword => "修改密码",
            OpType::CreateDept => "创建部门",
            OpType::UpdateDept => "更新部门",
            OpType::DeleteDept => "删除部门",
            OpType::QueryDept => "查询部门",
            OpType::CreateGroup => "创建群组",
            OpType::DeleteGroup => "删除群组",
            OpType::QueryGroup => "查询群组",
            OpType::AddGroupUser => "添加群组用户",
            OpType::DeleteGroupUser => "删除群组用户",
            OpType::Stat => "统计",
            OpType::UpdateStat => "更新统计",
        }
    }
}

/// 操作结果
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpResult {
    Success,
    Failed,
}

impl OpResult {
    pub fn to_chinese(&self) -> &'static str {
        match self {
            OpResult::Success => "成功",
            OpResult::Failed => "失败",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "disk_op_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 操作时间 (Unix 时间戳)
    pub op_time: i64,

    /// 操作用户
    #[sea_orm(column_type = "String(Some(32))")]
    pub username: String,

    /// 操作类型
    #[sea_orm(column_type = "String(Some(32))")]
    pub op_type: String,

    /// 操作描述
    #[sea_orm(column_type = "Text")]
    pub op_desc: String,

    /// 旧值 (用于记录修改前的值)
    #[sea_orm(column_type = "Text", nullable)]
    pub old_value: Option<String>,

    /// 操作结果
    #[sea_orm(column_type = "String(Some(16))")]
    pub result: String,

    /// 操作者IP
    #[sea_orm(column_type = "String(Some(64))", nullable)]
    pub ip: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 创建日志记录的辅助结构
#[derive(Clone, Debug)]
pub struct NewOpLog {
    pub username: String,
    pub op_type: OpType,
    pub op_desc: String,
    pub old_value: Option<String>,
    pub result: OpResult,
    pub ip: Option<String>,
}

impl NewOpLog {
    pub fn new(username: impl Into<String>, op_type: OpType, op_desc: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            op_type,
            op_desc: op_desc.into(),
            old_value: None,
            result: OpResult::Success,
            ip: None,
        }
    }

    pub fn with_result(mut self, result: OpResult) -> Self {
        self.result = result;
        self
    }

    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip = Some(ip.into());
        self
    }

    pub fn with_old_value(mut self, old_value: impl Into<String>) -> Self {
        self.old_value = Some(old_value.into());
        self
    }
}
