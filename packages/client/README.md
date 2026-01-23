# @opencode-vibe/client

该包是 Opencode Vibe Kanban 的前端应用，使用 React + Vite + TailwindCSS 构建，提供终端交互界面。

## 输入参数 (Inputs)

### `useOpencode` Hook

- `write(data: string)`
  - 类型: `string`
  - 说明: 将用户输入写入 WebSocket，转发到服务器的 PTY。

- `OPENCODE_SOCKET_URL` / `VITE_OPENCODE_SOCKET_URL`
  - 类型: `string`
  - 说明: 可选配置项，用于覆盖默认的 Socket 地址（默认 `http://localhost:3000`）。

- `onData(callback: (data: string) => void)`
  - 类型: `function`
  - 说明: 订阅服务端 `output` 事件，回调函数接收终端输出字符串。

### `Terminal` 组件

- Props: 当前版本无显式 Props，内部依赖 `useOpencode` 建立通信。

### `ChatView` 组件

- Props: 当前版本无显式 Props，内部依赖 `useOpencode` 订阅输出，并使用协议解析器处理消息。

### `InputBar` 组件

- Props: 当前版本无显式 Props，内部依赖 `useOpencode` 发送输入并读取连接状态。

## 输出参数 (Outputs)

- 终端渲染输出: 将服务端发送的字符流写入 `xterm.js` 实例并渲染到 UI。
- 消息面板输出: `ChatView` 将解析后的消息渲染为列表，支持按类型/日志级别过滤并显示分组标签。
- 输入栏输出: `InputBar` 发送用户命令并清空输入框。
- 连接状态: 通过 `isConnected` 状态向页面展示连接信息。

## 使用示例 (Usage Examples)

### 启动开发服务

```bash
npm run dev
```

### 终端输入输出示例

```text
User types: dir
InputBar emits: input -> "dir\r"
Server emits: output -> "<directory listing>"
```

## 内部逻辑 (Internal Logic)

1. `useOpencode` 在首次调用时创建 `socket.io-client` 单例连接，默认连接 `http://localhost:3000`，可通过环境变量覆盖。
2. `Terminal` 组件挂载时初始化 `xterm` 与 `FitAddon`，并监听容器尺寸变化。
3. 用户在终端内输入时，`xterm.onData` 触发并调用 `write`，将字符流发送至服务器。
4. 服务端通过 `output` 事件推送数据后，`Terminal` 会将内容写入 `xterm` 渲染区。
5. `ChatView` 订阅相同的输出事件，使用协议 `Parser` 转为带类型的消息对象并追加到消息列表。
6. 消息列表支持按类型筛选，在全量视图下按类型分组；日志视图支持按 level 过滤。
7. `InputBar` 提交时会对输入进行 trim，发送 `\r` 结尾的命令并清空输入框。

## 其他脚本

```bash
npm run build
npm test
# or
npx vitest run
```
