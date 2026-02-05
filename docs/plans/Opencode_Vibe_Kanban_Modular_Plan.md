# Project Plan: Opencode Vibe Kanban (Modular TDD Edition)

## 1. Architectural Strategy (架构策略)
采用 **Monorepo** 结构进行严格分层解耦，以支持**并行开发**和**测试驱动开发 (TDD)**。

### Directory Structure
```text
/
├── packages/
│   ├── protocol/       # [Shared] 核心协议定义、消息解析器 (Pure Logic)
│   ├── pty-manager/    # [Backend] 终端进程管理封装 (Node.js)
│   ├── server/         # [Backend] WebSocket 网关与 API 服务
│   └── client/         # [Frontend] React UI 应用
├── scripts/            # 构建与编排脚本
└── ...
```

## 2. Phase 1: Core Logic & TDD (核心逻辑与 TDD)
*目标：开发不依赖 UI 的底层核心模块，实现 100% 单元测试覆盖。*

### Task 1.1: Protocol Package (`packages/protocol`)
*   **Description**: 定义前后端通信的 Types 和 Opencode 输出解析逻辑。
*   **TDD Scenarios**:
    *   `OpencodeParser`: 输入 ANSI 原始流，输出结构化 `Message` 对象。
    *   `MessageTypes`: 定义 `Command`, `Log`, `StatusUpdate` 等接口。
*   **Deliverables**: `npm test` 通过，包含针对 ANSI 颜色去除、JSON 提取的测试用例。

### Task 1.2: PTY Manager (`packages/pty-manager`)
*   **Description**: 封装 `node-pty`，提供稳定的进程控制接口。
*   **TDD Scenarios**:
    *   `spawnProcess()`: 验证进程是否成功启动。
    *   `write()`: 验证向 stdin 写入数据。
    *   `onData()`: 验证能正确捕获 stdout 输出。
    *   `resize()`: 验证终端大小调整。
*   **Deliverables**: 一个独立的 npm 包，测试覆盖进程生命周期管理。

## 3. Phase 2: Server Implementation (服务端实现)
*目标：组装 Core 模块，提供 WebSocket 服务。*

### Task 2.1: Socket Gateway (`packages/server`)
*   **Description**: 基于 Express 和 Socket.io，集成 `pty-manager`。
*   **Integration Tests**:
    *   Client 连接后，Server 应自动 spawn 一个 shell。
    *   Client 发送指令 -> Server 写入 PTY -> PTY 输出 -> Server 转发回 Client。
*   **Deliverables**: 可运行的 WebSocket 服务器，支持多客户端连接（或单例互斥）。

## 4. Phase 3: Frontend Implementation (前端实现)
*目标：基于 React 的现代化 UI。*

### Task 3.1: State Management Hook (`useOpencode`)
*   **Description**: 封装 WebSocket 连接逻辑，处理重连、消息分发。
*   **TDD**: 使用 `renderHook` 测试连接状态变化和消息接收。

### Task 3.2: Components (UI 组件)
*   `TerminalView`: 封装 `xterm.js`，渲染原始流。
*   `ChatView`: 渲染 `protocol` 解析后的结构化消息。
*   `InputBar`: 移动端优化的指令输入栏。

## 5. Phase 4: CLI-Driven Kanban (CLI 驱动看板)
*目标：实现基于文件持久化的看板功能，支持 CLI 与 UI 双向操作。*

### Task 4.1: Protocol & State Definition
*   **Description**: 扩展协议以支持看板数据结构。
*   **Deliverables**:
    *   定义 `Task`, `Column`, `KanbanBoard` 接口。
    *   定义 `KANBAN_SYNC`, `KANBAN_MOVE`, `KANBAN_CREATE` 等 WebSocket 事件。

### Task 4.2: Server-Side Persistence
*   **Description**: 实现服务端状态管理。
*   **Deliverables**:
    *   `KanbanManager`: 启动时读取 `.opencode/kanban.json`，无则创建默认。
    *   支持文件监听：文件变更自动广播给 Client。
    *   支持指令解析：拦截 `/todo`, `/task` 等命令并更新 JSON。

### Task 4.3: Client Kanban UI
*   **Description**: 实现拖拽式看板界面。
*   **Deliverables**:
    *   `Board` 组件：集成 `@dnd-kit/core` 实现三列布局 (Todo/Doing/Done)。
    *   `TaskCard` 组件：展示任务详情。
    *   Optimistic UI：拖拽时立即更新 UI，失败则回滚。

### Task 4.4: Command Integration
*   **Description**: 前端输入栏支持看板指令。
*   **Deliverables**:
    *   拦截 `/task` 等指令，不发往 PTY，而是直接调用 Kanban API。

## 6. Execution Strategy with Parallel Agents (并行执行策略)

一旦项目脚手架搭建完成，可以立即启动 **Parallel Agents**：

*   **Agent A**: 负责 **Task 1.1 (Protocol)** —— 纯逻辑，无依赖。
*   **Agent B**: 负责 **Task 1.2 (PTY Manager)** —— Node.js 系统编程，无依赖。
*   **Agent C**: 负责 **Task 3.1 (Frontend Hooks)** —— 依赖 Protocol 定义的类型。

## 7. Verification Plan (验证计划)
*   **Unit Tests**: 各个 package 内部的 Jest/Vitest 测试。
*   **E2E Test**: 启动 Server，模拟 Client 发送 "echo hello"，验证链路通畅。

---
*Created by Antigravity via Writing Plans Skill*
