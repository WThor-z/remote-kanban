# OpenCode Vibe Kanban

AI-Powered Development with Visual Task Management

一个集成 AI 编程助手的可视化任务管理系统，让你可以通过看板界面管理开发任务，并一键启动 AI 来执行任务。

## Features

- **可视化看板**: 拖拽式三列布局 (To Do / Doing / Done)
- **AI 任务执行**: 点击任务卡片，一键启动 AI Agent 执行开发任务
- **实时同步**: 基于 WebSocket 的多客户端实时状态同步
- **命令驱动**: 支持 `/task` 命令快速创建任务
- **对话历史**: 保存并展示 AI 执行过程中的完整对话
- **文件持久化**: 任务数据自动保存到 `.opencode/kanban.json` 与 `.opencode/tasks/`（Node 模式），或 `.vk-data/tasks.json` 与 `.vk-data/kanban.json`（Rust 模式）

功能目录见 `docs/features/index.md`。

## Screenshots

```
┌─────────────────────────────────────────────────────────────┐
│                   OpenCode Vibe Kanban                      │
│            AI-Powered Development with Visual               │
│                   Task Management                           │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │    To Do    │  │    Doing    │  │    Done     │         │
│  ├─────────────┤  ├─────────────┤  ├─────────────┤         │
│  │ ┌─────────┐ │  │ ┌─────────┐ │  │ ┌─────────┐ │         │
│  │ │ Task 1  │ │  │ │ Task 2  │ │  │ │ Task 3  │ │         │
│  │ └─────────┘ │  │ └─────────┘ │  │ └─────────┘ │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ /task 任务标题 | 描述（可选）                 [Send] │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Node.js 18+
- pnpm 8+
- [OpenCode CLI](https://opencode.ai) (用于 AI 执行功能)

### Installation

```bash
# 克隆仓库
git clone https://github.com/WThor-z/remote-kanban.git
cd remote-kanban

# 安装依赖
pnpm install

# 构建所有包
pnpm build
```

### Running

```bash
# 终端 1: 启动服务端
cd packages/server && pnpm dev

# 终端 2: 启动客户端
cd packages/client && pnpm dev
```

Rust API 模式（REST 8081 + Socket.IO 8080）：

```bash
# 可选：使用 Rust API 后端
cd crates && cargo run -p api-server

# 可选：远程执行网关
cd services/agent-gateway && pnpm dev
# 需要环境变量：GATEWAY_SERVER_URL、GATEWAY_AUTH_TOKEN
```

访问 http://localhost:5173

## Project Structure

```text
crates/
├── core/           # Rust 核心模型与存储（crate: vk-core）
├── api-server/     # Rust REST + Socket.IO 后端
├── agent-runner/   # 任务执行与运行持久化
└── git-worktree/   # Git worktree 管理

packages/
├── protocol/       # 共享协议/类型与解析
├── server/         # Node Socket.IO 服务端
├── client/         # React 前端 (Vite + TailwindCSS)
└── pty-manager/    # PTY 进程管理 (已弃用，兼容保留)

services/
└── agent-gateway/  # 远程任务执行网关
```

## Usage

### 创建任务

使用命令输入框：

```
/task 添加用户登录功能 | 实现 JWT 认证和登录表单
```

或使用完整格式：

```
/task add 修复 Bug -- 首页加载过慢
```

### 执行任务

1. 点击任务卡片打开详情面板
2. 点击"开始执行"按钮
3. AI Agent 会自动分析任务并执行
4. 任务完成后自动移动到 Done 列

### 拖拽管理

- 拖拽任务卡片可在列之间移动
- 支持列内排序

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | React 18, Vite, TailwindCSS, @dnd-kit |
| Backend | Node.js, Express, Socket.IO |
| AI Integration | OpenCode HTTP API |
| Testing | Vitest |
| Language | TypeScript, Rust |

> 备注：Rust API 作为替代后端实现可选。

## API Reference

### WebSocket Events

| Event | Direction | Description |
|-------|-----------|-------------|
| `kanban:sync` | Server → Client | 同步看板状态 |
| `kanban:create` | Client → Server | 创建任务 |
| `kanban:move` | Client → Server | 移动任务 |
| `task:execute` | Client → Server | 执行任务 |
| `task:status` | Server → Client | 任务状态更新 |
| `task:message` | Server → Client | AI 消息 |

### REST API

Task cleanup endpoints:

| Method | Endpoint | Description |
|--------|----------|------|
| `DELETE` | `/api/tasks/{task_id}/runs/{run_id}` | Delete a specific run record |
| `DELETE` | `/api/tasks/{task_id}/runs` | Delete all run records for a task |
| `POST` | `/api/tasks/{task_id}/cleanup` | Clean up the task worktree |

### Commands

| Command | Description |
|---------|-------------|
| `/task <title>` | 创建任务 |
| `/task <title> \| <desc>` | 创建带描述的任务 |
| `/task add <title>` | 创建任务 (完整格式) |
| `/task move <id> <status>` | 移动任务 |
| `/task done <id>` | 标记完成 |
| `/task delete <id>` | 删除任务 |
| `/todo <title>` | 创建任务 (别名) |

## Development

### Running Tests

```bash
# 所有包
pnpm test

# 单个包
cd packages/client && pnpm test
cd packages/server && pnpm test
```

### Maintenance Commands

```bash
# 校验 docs/features 是否都在索引中注册
pnpm run check:docs:features

# 预览将清理的运行时目录（dry-run）
pnpm run cleanup:data

# 执行清理（会删除 .vk-data 与 crates/.vk-data 下的 runs/worktrees）
pnpm run cleanup:data:apply
```

## License

MIT

## Acknowledgments

- [OpenCode](https://opencode.ai) - AI 编程助手
- [dnd-kit](https://dndkit.com) - 拖拽库
- [Socket.IO](https://socket.io) - 实时通信
