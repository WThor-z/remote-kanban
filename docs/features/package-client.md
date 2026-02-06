# Node 包：client

## 概述
基于 React 的前端界面，负责任务看板管理与 AI 执行控制。

## 入口
- UI：`packages/client/src/App.tsx`
- API：通过 hooks 连接 REST 与 Socket.IO 端点
- CLI：`pnpm --filter client dev`

## 行为与边界
- 渲染看板、任务详情面板与执行控制区域。
- 通过 REST 获取任务创建/执行状态，通过 Socket.IO 做实时同步。

## 数据与存储影响
- 无（仅维护前端内存状态）。

## 权限与风险
- 依赖后端 REST 与 Socket.IO 服务可访问。

## 可观测性
- 在 UI 中展示网关状态与任务执行进度更新。

## 测试与验证
- 执行 `pnpm --filter client test`。

## 相关变更
- 依赖 `@opencode-vibe/protocol`。
