# @opencode-vibe/client

该包是 Opencode Vibe Kanban 的前端应用，使用 React + Vite + TailwindCSS 构建，提供终端交互界面。

## 输入参数 (Inputs)

### `useOpencode` Hook

- `write(data: string)`
  - 类型: `string`
  - 说明: 将用户输入写入 WebSocket，转发到服务器的 PTY。

- `onData(callback: (data: string) => void)`
  - 类型: `function`
  - 说明: 订阅服务端 `output` 事件，回调函数接收终端输出字符串。

### `Terminal` 组件

- Props: 当前版本无显式 Props，内部依赖 `useOpencode` 建立通信。

## 输出参数 (Outputs)

- 终端渲染输出: 将服务端发送的字符流写入 `xterm.js` 实例并渲染到 UI。
- 连接状态: 通过 `isConnected` 状态向页面展示连接信息。

## 使用示例 (Usage Examples)

### 启动开发服务

```bash
npm run dev
```

### 终端输入输出示例

```text
User types: dir
Client emits: input -> "dir\r"
Server emits: output -> "<directory listing>"
```

## 内部逻辑 (Internal Logic)

1. `useOpencode` 在首次调用时创建 `socket.io-client` 单例连接，默认连接 `http://localhost:3000`。
2. `Terminal` 组件挂载时初始化 `xterm` 与 `FitAddon`，并监听容器尺寸变化。
3. 用户在终端内输入时，`xterm.onData` 触发并调用 `write`，将字符流发送至服务器。
4. 服务端通过 `output` 事件推送数据后，`onData` 回调会将内容写入 `xterm` 渲染区。

## 其他脚本

```bash
npm run build
npm test
# or
npx vitest run
```
