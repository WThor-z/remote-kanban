# OpenCode Vibe Kanban: 从“OpenCode 调用器”到“开发驾驶舱”

*Created: 2026-01-31*
*Status: Draft*

## 0. 背景与问题（Brainstorming）

### 0.1 现状一句话

当前的 Kanban 更像是“把 OpenCode/Agent 跑起来的按钮集合”，而不是一个能让人长期管理真实开发工作的工作台。

### 0.2 典型使用路径为什么不成立

1. 用户创建卡片 -> 点击执行 -> 看到一些输出 -> 结束。
2. 缺失的关键环节：澄清/拆解、可观测执行、可回放结果、变更审查、反馈循环、收敛到可交付。

### 0.3 症状（用户感受层）

- “看板存在感很弱”：卡片字段少、状态少、对后续行为没有约束。
- “执行不可控/不可依赖”：缺少稳定的结构化时间线、没有 run 的概念，刷新/关闭就丢。
- “控制平面与工作界面混在一起”：TS/Rust 双后端 + 多端口默认不一致导致体验随机。
- “没有闭环”：没有 Review/Diff、没有验收标准、没有“下一步该干什么”的引导。

### 0.4 设计原则（借鉴 OpenClaw 的可取点）

- **控制平面（Gateway）与工作界面分离**：控制平面负责会话/事件/策略/执行路由；看板只做“工作流”。
- **Session/Run 是一等公民**：一次执行 = 一个可回放的 Run，具备状态机与事件流。
- **事件流是唯一事实来源**：UI 不拼 stdout；以结构化 `ExecutionEvent` 渲染时间线。
- **安全默认值是产品能力**：危险操作审批、权限分级、允许/禁止工具是显式且可配置的。
- **向导与体检（wizard/doctor）是体验的一部分**：让“从 0 到可用”是确定性的。

### 0.5 北极星体验（North Star）

用户打开 Kanban：

1) 能快速创建“可执行的任务”（含验收标准/风险/执行策略）。
2) 点击 Run 后看到“可信时间线”（工具调用、命令、文件变更、测试、结论）。
3) 结束后自动进入 Review（变更列表+diff+评论），一键把反馈回灌给 Agent 再跑。
4) 所有结果可回放、可追溯，并且能被再利用（模板/技能/策略）。

### 0.6 核心产品原语（建议落地为数据模型）

- **Task**：问题定义与验收标准（不是 prompt 文本）。
- **Run**：一次执行尝试（worktree、分支、开始/结束、状态、配置）。
- **Event**：Run 的结构化时间线（已存在 `ExecutionEvent` 协议）。
- **Artifact**：Run 产物（diff、测试报告、摘要、评论）。
- **Policy**：执行策略与安全策略（可在控制平面配置）。

### 0.7 关键决策（建议默认选项）

- **单一后端事实来源：Rust `crates/api-server` 作为 Gateway**（Socket.IO + REST），`services/agent-worker` 作为 Runner。
- **收敛前端到一个 socket**：前端默认连到 Gateway（不再默认连 TS `packages/server`）。
- **把 TS `packages/server` 降级为 Legacy/Playground**：短期可保留，但不占主 UI。

> 现实约束：当前前端 `AgentPanel` 依赖 `agent:*` 事件（仅 TS 后端实现）。如果切到 Rust，需要：A) 暂时隐藏该面板；或 B) 把 `agent:*` 事件也迁到 Rust。

## 1. 规划与拆解（Writing Plans）

### 1.1 目标（Goals）

- 让 Kanban 成为“开发驾驶舱”：能管理任务、驱动执行、呈现证据、支持 review 与反馈循环。
- 把“执行不确定性”产品化：事件流、策略、安全、可回放。
- 将系统从“双后端随机组合”收敛到一个 Gateway。

### 1.2 非目标（Non-goals）

- 不在第一阶段做完整的多渠道（Slack/Telegram）接入。
- 不在第一阶段做复杂的权限系统/账号体系（先单机单用户）。
- 不追求一次性做完 OpenClaw 同量级平台能力。

## 2. 里程碑（Milestones）

### M0 - 体验收敛：单一 Gateway + 配置一致性（1-2 天）

**目标**：默认开箱即用；日志/执行事件能稳定显示。

**工作项**

- 统一端口与默认配置：让前端默认连接到 Rust Socket + Rust REST。
  - 选项 A（推荐）：Rust Socket=3000、Rust REST=3001（与前端默认一致），TS server 改端口或停止启动。
  - 选项 B：保留 Rust=8080/8081，但前端默认改为 8080/8081。
- 在 UI 明确显示“当前连接的 Gateway / Worker / 数据目录”。
- 把 `AgentPanel` 移到“Playground/Legacy”页或折叠区（不再占主流程）。

**验收标准**

- 新用户按 README 启动后，Kanban 可同步、Run timeline 可实时滚动。
- `task:execution_event` 在 UI 可见且与 Task 对齐。

### M1 - Run 时间线成为主体验（2-4 天）

**目标**：任务详情以 Run 为中心；事件流可读、可过滤、可回放。

**前端交付**

- `packages/client/src/components/execution/ExecutionLogPanel.tsx`
  - 支持事件分组：按 Run（session_id）分段；在顶部显示 Run header（开始时间/分支/worktree）。
  - 支持筛选：只看 command/file_change/error；支持搜索。
  - 支持“固定重要事件”（status_changed、session_started/ended）。
- `packages/client/src/components/task/TaskDetailPanel.tsx`
  - Tabs 重构：Overview / Run / Review / Notes（chat 变为 Notes 或 run 注释）。
  - 默认打开 Run；执行中自动滚动；执行结束自动提示进入 Review。
- 修复类型与接口不一致（例如 `onSendInput` props 缺失等）。

**后端交付（Rust）**

- 为每次执行生成 `run_id`（可沿用 session_id），并**持久化事件**：
  - 建议目录：`.vk-data/runs/{taskId}/{runId}.jsonl`（append-only，易于回放与调试）。
  - 新增 REST：`GET /api/tasks/:id/runs`、`GET /api/tasks/:id/runs/:runId/events`。
- Socket.IO 增强：支持按 task 订阅（room：`task:{id}`），避免全量广播。

**验收标准**

- 关闭页面再打开仍能看到上一次 Run 的时间线（回放）。
- 同一个 task 多次执行会形成多个 Run，可选择查看。

### M2 - Task 模型“可管理”（2-3 天）

**目标**：任务不再只是标题+描述，具备“执行所需信息”和“完成定义”。

**任务字段（最小集合）**

- acceptanceCriteria（文本或 checklist）
- agentType / baseBranch / riskLevel（low/medium/high）
- repoContext（可选：路径/模块/相关文件）

**交付**

- 扩展 Rust `Task`（`crates/core/src/task/model.rs`）与 REST `CreateTaskRequest`/`TaskResponse`。
- `CreateTaskModal` 增加“验收标准（可选）+ 风险级别 + 创建并立即执行”。
- 看板卡片信息密度提升：展示 priority/agent/baseBranch/最后一次 run 状态。

**验收标准**

- 任务创建后无需额外操作即可正确执行（agent/baseBranch 有默认值）。
- 任务详情页展示“完成定义”，并在 Run 完成后提示用户逐条确认。

### M3 - Review 闭环（3-6 天）

**目标**：执行结束自动进入 Review：变更可视化 + 评论 + 一键反馈继续跑。

**交付**

- 后端：实现 worktree diff 提取与 API（与现有迁移计划 M-4.1/M-4.2 对齐）
  - `GET /api/tasks/:id/changes`
  - `GET /api/tasks/:id/changes/:path`
  - `POST /api/tasks/:id/comments`
  - `POST /api/tasks/:id/feedback`
- 前端：DiffViewer + 行级评论 + Send feedback。

**验收标准**

- Run 完成后可查看变更列表与 diff。
- 评论可发送到 Agent，触发新 Run，并在时间线中体现“反馈来源”。

### M4 - Onboarding/Doctor（可选，1-3 天）

**目标**：把“配置正确性”变成产品体验，减少随机坏掉。

**交付**

- `GET /api/doctor`：检查 Git 仓库、baseBranch 是否存在、Worker 是否可达、数据目录权限、端口冲突等。
- 前端首次启动引导页：一键运行 doctor，给出修复建议。

## 3. 技术改动清单（按代码位置）

### 前端

- Socket 连接策略：`packages/client/src/hooks/useOpencode.ts` 与 `packages/client/src/hooks/useKanban.ts`
  - 目标：默认连接到 Rust Gateway；必要时拆分为 `useGatewaySocket()` 与 `useLegacySocket()`。
- 执行事件：`packages/client/src/hooks/useExecutionEvents.ts`
  - 目标：支持按 run 订阅 + 拉取历史（REST）。
- 任务详情：`packages/client/src/components/task/TaskDetailPanel.tsx`
  - 目标：Run-first 信息架构。

### Rust Gateway

- 端口与注释一致：`crates/api-server/src/main.rs`
- Socket.IO 事件（kanban/task/execute）：`crates/api-server/src/socket.rs`
- 执行桥接与事件：`crates/api-server/src/routes/executor.rs`
- 执行器与 worktree：`crates/agent-runner/src/executor.rs`、`crates/git-worktree/src/worktree.rs`
- 任务存储：`crates/core/src/task/*`
- 看板存储与 task 同步：`crates/core/src/kanban/store.rs`

### Node Worker

- SSE 输出事件协议稳定化：`services/agent-worker/src/index.ts`
  - 目标：输出结构化事件（不仅是 log 文本），便于 Rust 解析为 `AgentEvent`。

## 4. 风险与应对

- 双后端并存导致体验分裂：优先完成 M0（默认只连接 Gateway）。
- 事件格式不稳定（Worker 输出只是一堆 log）：先约定最小结构（status/log/command/file）。
- 历史/回放数据增长：run events 使用 jsonl + 按 task 分目录 + 支持清理策略。

## 5. 验证计划

- Rust：`cargo test`（重点覆盖 run 持久化、diff 提取、executor 状态机）。
- Frontend：`pnpm -C packages/client test`（事件渲染、过滤、回放加载）。
- 手工脚本：创建任务 -> 执行 -> 查看 timeline -> 查看 diff -> 评论 -> feedback -> 产生新 Run。
