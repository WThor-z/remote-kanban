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

## 部署指南

### 1) 本地部署（单机调试）

#### 前置条件

- Node.js 18+
- pnpm 8+
- Rust stable（用于 `api-server`）
- [OpenCode CLI](https://opencode.ai)（用于 Gateway 执行任务）

#### 步骤

```bash
# 1. 克隆仓库
git clone https://github.com/WThor-z/remote-kanban.git
cd remote-kanban

# 2. 安装主项目依赖（包含 packages/*）
pnpm install

# 3. 安装 Gateway 依赖（agent-gateway 不在 workspace 内）
(cd services/agent-gateway && pnpm install)
```

```bash
# 4. 启动 Rust API（REST 8081 + Socket.IO 8080）
(cd crates && cargo run -p api-server)
```

```bash
# 5. 启动 UI（新终端）
cd packages/client

pnpm dev
```

```bash
# 6. （可选）本地启动 Gateway（新终端，无需额外配置）
cd services/agent-gateway

pnpm dev
```

#### 一键启动命令（推荐）

```bash
# 一键本地启动（Rust + UI + Gateway 本地连接）
pnpm run dev:local
# 或：pnpm run start:local

# 一键启动应用（Rust + UI）
pnpm run dev:app
# 或：pnpm run start:app

# 一键启动 Gateway（连接本地 API）
pnpm run dev:gateway:local

# 一键启动 Gateway（连接云端 API，读取仓库根目录 .env.gateway）
pnpm run dev:gateway:cloud
```

本地默认值（零配置）：

- UI 默认连接 `http://localhost:8080`（Socket）和 `http://localhost:8081`（REST）
- Gateway 默认连接 `ws://127.0.0.1:8081`
- Gateway 与 API 默认认证 token 都是 `dev-token`

访问 `http://localhost:5173`。

### 2) 服务器部署（Rust + UI）

以下示例以 Linux + systemd + Nginx 为例。

#### 2.1 构建

```bash
# 仓库根目录
pnpm install

# 构建 Rust API
(cd crates && cargo build -p api-server --release)
```

```bash
# 构建 UI（替换为你的域名）
(cd packages/client && \
  VITE_OPENCODE_SOCKET_URL=https://kanban.example.com \
  VITE_RUST_API_URL=https://kanban.example.com \
  pnpm build)
```

#### 2.2 统一配置文件 + 启动 Rust API（systemd）

先创建统一配置文件 `/etc/opencode-vibe.env`（同一台机器上 API/Gateway 可共用）：

```bash
sudo tee /etc/opencode-vibe.env >/dev/null <<'EOF'
GATEWAY_AUTH_TOKEN=replace-with-strong-token
VK_DATA_DIR=/var/lib/opencode-vibe
VK_REPO_PATH=/srv/projects
EOF
```

创建 `/etc/systemd/system/opencode-api.service`：

```ini
[Unit]
Description=OpenCode Vibe Kanban API Server
After=network.target

[Service]
Type=simple
WorkingDirectory=/opt/remote-kanban
EnvironmentFile=-/etc/opencode-vibe.env
ExecStart=/opt/remote-kanban/crates/target/release/api-server
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```

```bash
sudo mkdir -p /var/lib/opencode-vibe
sudo systemctl daemon-reload
sudo systemctl enable --now opencode-api
```

#### 2.3 发布 UI（Nginx）

将 `packages/client/dist` 发布到 Nginx 站点目录（如 `/var/www/opencode-vibe`），并配置反向代理：

```nginx
server {
  listen 80;
  server_name kanban.example.com;

  root /var/www/opencode-vibe;
  index index.html;

  location / {
    try_files $uri /index.html;
  }

  # Socket.IO (8080)
  location /socket.io/ {
    proxy_pass http://127.0.0.1:8080/socket.io/;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
  }

  # REST API (8081)
  location /api/ {
    proxy_pass http://127.0.0.1:8081/api/;
    proxy_set_header Host $host;
  }

  location = /health {
    proxy_pass http://127.0.0.1:8081/health;
    proxy_set_header Host $host;
  }

  # Gateway WebSocket (8081)
  location /agent/ws {
    proxy_pass http://127.0.0.1:8081/agent/ws;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
  }
}
```

### 3) 主机部署 Gateway（远程执行机）

在执行机上部署 `services/agent-gateway`，并保持常驻。

#### 3.1 安装与构建

```bash
git clone https://github.com/WThor-z/remote-kanban.git
cd remote-kanban/services/agent-gateway
pnpm install
pnpm build
```

> 首次部署前请在该主机完成 `opencode` 登录，确保模型可用。

#### 3.2 配置环境变量

最小必填（推荐放到 `/etc/opencode-vibe.env`）：

- `GATEWAY_SERVER_URL`：例如 `wss://kanban.example.com`
- `GATEWAY_AUTH_TOKEN`：必须与 API 机器上的 `GATEWAY_AUTH_TOKEN` 一致

```bash
sudo tee /etc/opencode-vibe.env >/dev/null <<'EOF'
GATEWAY_SERVER_URL=wss://kanban.example.com
GATEWAY_AUTH_TOKEN=replace-with-strong-token
EOF
```

本地手动连接云端时，可先将 `.env.gateway.example` 复制为 `.env.gateway` 并填写真实值，再执行：

```bash
pnpm run dev:gateway:cloud
```

推荐配置：

- `GATEWAY_CWD`：默认执行目录
- `GATEWAY_ALLOWED_PROJECT_ROOTS`：允许执行的项目根目录（逗号分隔）

可选配置：

- `GATEWAY_HOST_ID`、`GATEWAY_HOST_NAME`、`GATEWAY_MAX_CONCURRENT`、`OPENCODE_PORT`

#### 3.3 作为 systemd 服务启动

创建 `/etc/systemd/system/opencode-gateway.service`：

```ini
[Unit]
Description=OpenCode Agent Gateway
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=/opt/remote-kanban/services/agent-gateway
EnvironmentFile=-/etc/opencode-vibe.env
Environment=GATEWAY_HOST_ID=worker-01
Environment=GATEWAY_HOST_NAME=Worker 01
Environment=GATEWAY_MAX_CONCURRENT=2
Environment=GATEWAY_CWD=/srv/projects
Environment=GATEWAY_ALLOWED_PROJECT_ROOTS=/srv/projects
ExecStart=/usr/bin/env pnpm start
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now opencode-gateway
```

若不使用 systemd，也可以手动一键启动（需先 `pnpm --dir services/agent-gateway build`）：

```bash
pnpm run start:gateway:cloud
```

### 4) 部署后检查

```bash
# API 健康检查
curl http://127.0.0.1:8081/health

# 查看已连接的 Gateway 主机
curl http://127.0.0.1:8081/api/hosts
```

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

## License

MIT

## Acknowledgments

- [OpenCode](https://opencode.ai) - AI 编程助手
- [dnd-kit](https://dndkit.com) - 拖拽库
- [Socket.IO](https://socket.io) - 实时通信

## Agent Memory

项目已集成 Agent Memory 模块（以 Gateway 为主链路）：

- Gateway 会在任务执行前注入相关记忆上下文。
- Gateway 会在任务执行后抽取并沉淀可复用记忆（规则优先，LLM 兜底）。
- 记忆按 `hostId` 和可选 `projectId` 隔离。
- 存储模式支持本地、中央、双写和全关。
- 前端提供独立 `Memory` 页面，用于设置、检索、CRUD 与开关控制。

### Memory REST API

- `GET /api/memory/settings`
- `PATCH /api/memory/settings`
- `GET /api/memory/items`
- `POST /api/memory/items`
- `PATCH /api/memory/items/{id}`
- `DELETE /api/memory/items/{id}`

### Gateway 环境变量

- `MEMORY_ENABLE`
- `MEMORY_GATEWAY_STORE_ENABLE`
- `MEMORY_RUST_STORE_ENABLE`
- `MEMORY_AUTO_WRITE_ENABLE`
- `MEMORY_PROMPT_INJECTION_ENABLE`
- `MEMORY_INJECTION_TOKEN_BUDGET`
- `MEMORY_RETRIEVAL_TOP_K`
- `MEMORY_LLM_EXTRACT_ENABLE`
- `MEMORY_DATA_DIR`
