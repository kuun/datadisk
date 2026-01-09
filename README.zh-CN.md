## Datadisk

Datadisk 是一款高性能的企业级网盘系统，提供安全可控的文件管理、权限体系、审计能力与协作流程。后端基于 Rust + Axum + SeaORM，前端基于 React + Vite，适用于企业内部文件存储与共享场景。

## 功能介绍

- 用户、部门、角色与权限控制（Casbin）
- 文件与目录的创建、删除、移动、复制、重命名
- 流式上传、单文件下载与批量打包下载
- 文件预览与压缩包预览
- 最近访问、任务管理与审计日志
- WebSocket 推送通知
- OnlyOffice 在线编辑（可选）

## 快速开始

### 依赖准备

- Rust 1.70+（建议使用 rustup）
- Node.js 18+
- PostgreSQL 12+

### 后端启动

1. 复制并修改配置文件：
   ```bash
   cp etc/datadisk_example.toml etc/datadisk.toml
   ```
2. 修改 `etc/datadisk.toml`：
   - `addr`：服务监听地址
   - `root_dir`：文件存储根目录
   - `config_dir`：配置目录，需可写（用于保存 `db.toml` 和 `sys_inited`）
3. 启动后端：
   ```bash
   cargo run -- -config etc/datadisk.toml
   ```
4. 访问初始化页面完成数据库与管理员初始化：
   - `http://localhost:8080/setup.html`

### 前端启动

```bash
cd webapp
npm install
npm run dev
```

前端默认代理 `http://127.0.0.1:8080`，打开 `http://localhost:5173` 进行开发。

### 构建部署

```bash
cd webapp
npm run build
```

构建产物输出至 `webapp/dist`，后端会自动作为静态资源提供服务。

## 开发指导

### 后端开发

- 本地运行：
  ```bash
  cargo run -- -config etc/datadisk.toml
  ```
- 运行测试：
  ```bash
  cargo test
  ```
- 日志级别：
  ```bash
  RUST_LOG=debug cargo run -- -config etc/datadisk.toml
  ```

### 前端开发

- 启动开发服务器：
  ```bash
  cd webapp
  npm run dev
  ```
- 代码检查：
  ```bash
  npm run lint
  ```

## 目录结构

- `src/`：后端代码（Axum + SeaORM）
- `webapp/`：前端代码（React + Vite）
- `etc/`：配置与初始化文件
- `testdir/`：测试用数据与示例配置

## 许可证

Apache License 2.0。详见 `LICENSE`。
