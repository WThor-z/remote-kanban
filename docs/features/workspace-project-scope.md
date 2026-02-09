# 工作区与项目作用域

## 概述
该功能提供工作区（workspace）与项目（project）双层作用域管理，让任务的可见范围、创建归属和执行上下文保持一致，避免跨仓库误操作。

## 入口
- UI：`packages/client/src/App.tsx` 的 workspace scope 下拉；`packages/client/src/components/task/CreateTaskModal.tsx` 的 workspace/project 级联选择
- API：`GET/POST/PATCH /api/workspaces`、`GET /api/projects?workspaceId=...`、`GET /api/tasks?workspaceId=...`
- CLI：无

## 行为与边界
- 支持在应用级切换 `workspace` 作用域，并据此过滤看板任务与创建弹窗可选项目。
- 新建任务必须绑定 `projectId`，任务执行时后端会基于项目绑定自动注入 `workspace/project` 上下文。
- 执行前会校验任务与项目的 workspace 一致性；不一致返回 `409`，缺失任务 workspace 时后端会自动回填。
- 当 scope 为空时视为不限制工作区（展示全部可见任务）。

## 数据与存储影响
- 前端通过 `localStorage` 键 `vk-active-workspace-scope` 持久化当前 scope。
- 后端在 `.vk-data/workspaces.json` 与 `.vk-data/projects.json` 中保存作用域实体与绑定关系。

## 权限与风险
- 依赖后端工作区/项目接口可访问。
- 错误的项目绑定会直接影响任务执行目录与目标主机，需保证项目配置准确。

## 可观测性
- 前端在顶部显示当前 workspace 作用域，并在作用域切换后触发任务列表刷新。
- 后端在执行链路返回明确错误码（如 `404/409/422`）定位绑定问题。

## 测试与验证
- 执行 `pnpm --filter client test` 验证 workspace scope 过滤与创建弹窗级联行为。
- 执行 `cargo test -p api-server` 验证 workspace/project 路由与执行一致性校验。

## 相关变更
- `packages/client/src/App.tsx`
- `packages/client/src/components/task/CreateTaskModal.tsx`
- `crates/api-server/src/routes/workspace.rs`
- `crates/api-server/src/routes/project.rs`
- `crates/api-server/src/routes/executor.rs`
