# @opencode-vibe/server

该包提供 Opencode Vibe Kanban 的后端服务，负责 WebSocket 连接管理与 PTY 会话编排。

## 输入参数 (Inputs)

### WebSocket 事件

- `input` (Client -> Server)
  - 类型: `string`
  - 说明: 客户端终端输入的原始字符流，例如键盘输入、命令文本、控制字符等。

## 输出参数 (Outputs)

### WebSocket 事件

- `output` (Server -> Client)
  - 类型: `string`
  - 说明: PTY 进程输出的原始字符流，包含命令输出、提示符、错误信息等。

## 使用示例 (Usage Examples)

### 启动服务

```bash
npm run build --workspaces
npm start
```

### 客户端交互示例

```text
Client -> Server: "dir\r"
Server -> Client: "<directory listing>"
```

## 内部逻辑 (Internal Logic)

1. 客户端通过 `socket.io` 建立连接后，服务端创建一个新的 PTY 会话。
2. 根据操作系统选择 Shell:
   - Windows: `powershell.exe`
   - macOS/Linux: `bash`
3. 服务端监听 `input` 事件，将数据写入 PTY 的 stdin。
4. 服务端订阅 PTY 的 `onData` 输出流，并通过 `output` 事件推送给客户端。
5. 单例互斥策略: 新客户端连接时会关闭旧会话，确保仅保留一个活动 PTY。
6. 客户端断开连接时，销毁 PTY 进程并释放资源。
