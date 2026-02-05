# OpenCode Vibe Kanban

AI-Powered Development with Visual Task Management

一个集成 AI 编程助手的可视化任务管理系统，让你可以通过看板界面管理开发任务，并一键启动 AI 来执行任务。

## Features

- **可视化看板**: 拖拽式三列布局 (Todo / Doing / Done)
- **AI 任务执行**: 点击任务卡片，一键启动 AI Agent 执行开发任务
- **实时同步**: 基于 WebSocket 的多客户端实时状态同步
- **命令驱动**: 支持 `/task` 命令快速创建任务
- **对话历史**: 保存并展示 AI 执行过程中的完整对话
- **文件持久化**: 任务数据自动保存到 `.opencode/kanban.json`

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

访问 http://localhost:5173

## Project Structure

```
packages/
├── protocol/       # 共享类型定义和工具函数
├── server/         # WebSocket 服务端 (Express + Socket.io)
├── client/         # React 前端 (Vite + TailwindCSS)
└── pty-manager/    # PTY 进程管理 (已弃用，保留兼容)
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
| Backend | Node.js, Express, Socket.io |
| AI Integration | OpenCode HTTP API |
| Testing | Vitest |
| Language | TypeScript |

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

## License

MIT

## Acknowledgments

- [OpenCode](https://opencode.ai) - AI 编程助手
- [dnd-kit](https://dndkit.com) - 拖拽库
- [Socket.io](https://socket.io) - 实时通信
