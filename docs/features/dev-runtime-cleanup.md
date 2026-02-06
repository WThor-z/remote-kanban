# 开发能力：运行时数据清理命令

## 概述
提供统一的运行时数据清理能力，支持 dry-run 预览与 apply 执行，避免 runs/worktrees 长期堆积导致磁盘占用持续增长。

## 入口
- UI：无
- API：无
- CLI：
  - `pnpm run cleanup:data`（dry-run）
  - `pnpm run cleanup:data:apply`（实际删除）
  - `node scripts/cleanup-runtime.mjs --runs-only`
  - `node scripts/cleanup-runtime.mjs --worktrees-only`

## 行为与边界
- 默认目标目录：`.vk-data/runs`、`.vk-data/worktrees`、`crates/.vk-data/runs`、`crates/.vk-data/worktrees`。
- `dry-run` 只打印将删除内容，不做实际删除。
- `apply` 会递归删除目标目录；当包含 `worktrees` 时会附带执行 `git worktree prune --expire now --verbose`。
- 参数 `--runs-only` 与 `--worktrees-only` 互斥，冲突时直接失败。

## 数据与存储影响
- 删除运行过程持久化目录及工作树目录，属于不可逆操作。

## 权限与风险
- 需要对目标目录有删除权限。
- `--apply` 为破坏性操作，建议先执行 dry-run。

## 可观测性
- 脚本输出会明确标记 `DRY-RUN` 或 `APPLY`，并列出具体目录。

## 测试与验证
- 执行 `pnpm run test:scripts`（覆盖 dry-run、apply、参数冲突场景）。

## 相关变更
- `scripts/cleanup-runtime.mjs`
- `scripts/__tests__/cleanup-runtime.test.mjs`
- `package.json`（`cleanup:data` 与 `cleanup:data:apply`）
