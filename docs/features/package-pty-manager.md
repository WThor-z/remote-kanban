# Node 包：pty-manager

## 概述
用于启动和管理终端进程的 Node PTY 封装（已弃用）。

## 入口
- UI：无
- API：NPM 包 `@opencode-vibe/pty-manager`
- CLI：无

## 行为与边界
- 提供 PTY 进程启动与基础 I/O 操作能力。
- Windows 下使用 `useConpty = false` 以提升兼容性。
- 该模块已弃用，当前仅为兼容历史链路而保留（与根 README 说明一致）。

## 数据与存储影响
- 无。

## 权限与风险
- 会启动本地进程，需要操作系统允许相关权限。

## 可观测性
- 无。

## 测试与验证
- 执行 `pnpm --filter @opencode-vibe/pty-manager test`。

## 相关变更
- 在 `packages/server` 的依赖中被引用。
