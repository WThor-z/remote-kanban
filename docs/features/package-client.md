# Node 包：client

## 概述
基于 React 的前端界面，负责任务看板管理与 AI 执行控制，并提供 workspace 作用域下的任务/项目筛选体验。

## 入口
- UI：`packages/client/src/App.tsx`
- API：通过 hooks 连接 REST 与 Socket.IO 端点
- CLI：`pnpm --filter client dev`

## 行为与边界
- 渲染看板、任务详情面板与执行控制区域。
- 支持 app 级 workspace scope 选择，并将 scope 传递到：
  - 任务创建弹窗（workspace -> project 级联筛选）
  - 看板任务可见范围过滤
- `CreateTaskModal` 支持 `defaultWorkspaceId` 与上下文 fallback 机制。
- `useProjects` / `useTaskApi` / `useTaskRuns` 支持 workspace/project 过滤与上下文字段显示。

## 数据与存储影响
- 使用 localStorage 持久化 app 级 workspace scope（键：`vk-active-workspace-scope`）。
- 其余状态仍以前端内存态为主。

## 权限与风险
- 依赖后端 REST 与 Socket.IO 服务可访问。
- workspace scope 切换会影响当前看板与弹窗可见数据范围，需要注意用户预期一致性。

## 可观测性
- 在 UI 中展示网关状态、任务执行进度与当前 workspace 作用域。
- 通过前端测试覆盖 scope 切换到 modal/filter 的关键链路。

## 测试与验证
- 执行 `pnpm --filter client test`。

## 相关变更
- 依赖 `@opencode-vibe/protocol`。
