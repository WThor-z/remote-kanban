# Node 包：protocol

## 概述
提供看板事件与 Agent 输出消息的共享 TypeScript 类型与解析工具。

## 入口
- UI：无
- API：NPM 包 `@opencode-vibe/protocol`
- CLI：无

## 行为与边界
- 导出看板事件类型、Agent 类型与执行相关辅助类型。
- 提供基础的 Agent 输出消息解析能力。
- 不负责网络 I/O，也不负责数据持久化。

## 数据与存储影响
- 无。

## 权限与风险
- 无。

## 可观测性
- 无。

## 测试与验证
- 执行 `pnpm --filter @opencode-vibe/protocol test`。

## 相关变更
- 被 `packages/client` 与 `packages/server` 依赖。
