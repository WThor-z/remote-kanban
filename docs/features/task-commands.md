# 任务命令

## 概述
用户可以在不离开看板页面的情况下，通过命令输入框快速创建和管理任务。该能力为日常开发提供了低成本、快捷的任务操作入口。

## 入口
- UI：命令输入框
- API：无
- CLI：无

## 行为与边界
- 支持通过 `/task <title> | <desc>` 或 `/task add <title>` 创建任务。
- 支持通过 `/task move` 与 `/task done` 移动或完成任务。
- 支持通过 `/task delete <id>` 删除任务。
- 仅支持 `/task` 与 `/todo` 变体，其他输入不会触发任务创建。

## 数据与存储影响
- 更新并持久化任务数据到 `.opencode/kanban.json`。

## 权限与风险
- 无（仅本地任务操作）。

## 可观测性
- 客户端发出 `kanban:create` / `kanban:move`，服务端广播 `kanban:sync` 以实现同步。

## 测试与验证
- 在输入框执行 `/task` 创建任务，确认任务出现在看板中。

## 相关变更
- 见 `README` 的命令章节（Commands）。
