# Git Worktree 架构设计文档

> 创建日期: 2025-02-03
> 状态: 设计中

## 1. 概述

### 1.1 核心目标
- **并行隔离**: 多个任务可以同时执行，互不干扰
- **安全回滚**: 任务失败时可以安全回滚，不影响主分支
- **版本追溯**: 每个任务的修改都有完整的 Git 历史

### 1.2 设计原则
- 一个项目绑定一个 Gateway（一对一关系）
- Gateway 负责所有 Git 操作，中央服务器不存放代码
- 默认自动合并，冲突时保留分支让用户处理

---

## 2. 整体架构

```
用户（浏览器）
     │
     ▼
┌──────────────────┐
│   中央服务器       │  ← 只负责调度和管理，不存放代码
│  (API + 前端)     │
└──────────────────┘
     │ 分发任务
     ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│ Gateway A │  │ Gateway B │  │ Gateway C │
│ (主机 1)  │  │ (主机 2)  │  │ (主机 3)  │
├──────────┤  ├──────────┤  ├──────────┤
│ 项目 X    │  │ 项目 Y    │  │ 项目 Z    │  ← 每个项目固定在一个Gateway
│ Worktree │  │ Worktree │  │ Worktree │
│ Git 操作  │  │ Git 操作  │  │ Git 操作  │
└────┬─────┘  └────┬─────┘  └────┬─────┘
     │ push        │ push        │ push
     ▼             ▼             ▼
┌─────────────────────────────────────────┐
│      远程 Git 仓库 (GitHub/GitLab)       │
└─────────────────────────────────────────┘
```

### 2.1 目录结构

```
Gateway 主机上:
my-project/
├── .git/                         <- 共享的 Git 数据
├── .worktrees/                   <- 任务工作区目录
│   ├── fix-login-2025-02-03/     <- 任务 A 的工作区
│   └── add-feature-2025-02-03/   <- 任务 B 的工作区
├── src/                          <- 主工作区 (main 分支)
└── .vk-data/                     <- 运行数据

分支结构:
main ─────────────────────────────> 主分支
  ├── task/fix-login-2025-02-03    任务 A 分支
  └── task/add-feature-2025-02-03  任务 B 分支
```

---

## 3. 任务生命周期

```
1. 创建任务
   └── 用户填写: 标题、描述、分支名(可选)、合并策略、冲突策略

2. 开始执行
   ├── 生成分支名: task/{slug}-{date}
   ├── 创建 worktree: .worktrees/{slug}-{date}
   ├── 切换到新分支
   └── Agent 在 worktree 中执行任务

3. 执行完成
   ├── 自动 commit: "task: {title}"
   └── 标记状态: "待合并"

4. 合并阶段 (根据 merge_strategy)
   ├── auto (自动合并)
   │   ├── 尝试 merge 到 main
   │   │   ├── 成功 → git push, 标记完成
   │   │   └── 冲突 → 根据 conflict_strategy:
   │   │       ├── ai_resolve → 让 AI 解决冲突
   │   │       └── keep_branch → 通知用户手动处理
   │   └── 更新 git_status
   │
   └── manual (手动确认)
       └── 等待用户点击「合并」按钮

5. 清理 (用户手动触发)
   ├── 删除 worktree
   └── 删除分支
```

---

## 4. 数据模型

### 4.1 Project 模型 (新增)

```rust
// crates/vk-core/src/project.rs

pub struct Project {
    pub id: Uuid,
    pub name: String,                    // 项目名称，如 "vibe-kanban"
    pub local_path: String,              // 本地路径（Gateway 机器上）
    pub remote_url: Option<String>,      // 远程仓库 URL（仅供显示）
    pub default_branch: String,          // 默认分支，默认 "main"
    pub gateway_id: Uuid,                // 绑定的 Gateway（必须）
    pub worktree_dir: String,            // Worktree 目录，默认 ".worktrees"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// 唯一约束: (gateway_id, local_path)
// 同一个 Gateway 上的同一个路径 = 同一个项目
```

### 4.2 Task 模型 (更新)

```rust
// crates/vk-core/src/task.rs

pub struct Task {
    // === 现有字段 ===
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub column_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    
    // === 新增字段 ===
    pub project_id: Uuid,                    // 所属项目（必须）
    pub branch_name: Option<String>,         // 自定义分支名（可选）
    pub merge_strategy: MergeStrategy,       // 合并策略
    pub conflict_strategy: ConflictStrategy, // 冲突策略
    pub git_status: GitStatus,               // Git 状态
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MergeStrategy {
    #[default]
    Auto,    // 完成后自动合并
    Manual,  // 等待用户确认
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ConflictStrategy {
    AiResolve,   // AI 尝试解决冲突
    #[default]
    KeepBranch,  // 保留分支，通知用户
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GitStatus {
    #[default]
    None,           // 未开始 Git 操作
    WorktreeCreated,// Worktree 已创建
    Committed,      // 已提交
    Merged,         // 已合并到主分支
    Pushed,         // 已推送到远程
    Conflict,       // 合并冲突
    Failed,         // Git 操作失败
}
```

### 4.3 默认值

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `merge_strategy` | `Auto` | 任务完成后自动合并 |
| `conflict_strategy` | `KeepBranch` | 冲突时保留分支让用户处理 |
| `git_status` | `None` | 未开始 Git 操作 |
| `branch_name` | `None` | 自动生成: `task/{slug}-{date}` |

---

## 5. Gateway 自动注册流程

```
┌─────────────────────────────────────────────────────────────────┐
│                    Gateway 启动流程                              │
└─────────────────────────────────────────────────────────────────┘

1. Gateway 启动
   │
   ▼
2. 检测项目路径
   ├── 有 --project-path 参数 → 使用指定路径
   └── 无参数 → 使用当前工作目录 (cwd)
   │
   ▼
3. 验证 Git 仓库
   ├── 检查 .git 目录是否存在
   ├── 失败 → 报错退出: "Not a git repository"
   └── 成功 → 继续
   │
   ▼
4. 收集项目信息
   ├── name: 从目录名获取，或 --project-name 参数
   ├── local_path: 项目绝对路径
   ├── remote_url: 从 `git remote get-url origin` 获取
   ├── default_branch: 从 `git symbolic-ref refs/remotes/origin/HEAD` 获取
   └── worktree_dir: 默认 ".worktrees"
   │
   ▼
5. 连接中央服务器
   │
   ▼
6. 发送注册请求
   POST /api/gateways/register
   {
     "gateway_id": "uuid",
     "gateway_name": "my-macbook",
     "project": {
       "name": "vibe-kanban",
       "local_path": "/Users/me/projects/vibe-kanban",
       "remote_url": "git@github.com:user/vibe-kanban.git",
       "default_branch": "main"
     }
   }
   │
   ▼
7. 中央服务器处理
   ├── 查找是否已有 (gateway_id, local_path) 的 Project
   │   ├── 有 → 更新信息
   │   └── 无 → 创建新 Project
   └── 返回 project_id 给 Gateway
   │
   ▼
8. Gateway 保存 project_id，开始正常工作
```

### 5.1 Gateway 启动命令

```bash
# 方式 1: 在项目目录中启动（推荐）
cd /path/to/my-project
agent-gateway

# 方式 2: 指定项目路径
agent-gateway --project-path /path/to/my-project

# 方式 3: 指定项目名称
agent-gateway --project-path /path/to/my-project --project-name "My App"
```

---

## 6. API 接口设计

### 6.1 项目相关

```
GET  /api/projects              # 获取项目列表
GET  /api/projects/:id          # 获取项目详情
POST /api/projects              # 创建项目（手动创建）
PUT  /api/projects/:id          # 更新项目配置
```

### 6.2 任务相关（更新）

```
POST /api/tasks                 # 创建任务（需要 project_id）
POST /api/tasks/:id/merge       # 手动触发合并
POST /api/tasks/:id/push        # 手动推送分支
POST /api/tasks/:id/cleanup     # 清理分支和 worktree
GET  /api/tasks/:id/git-status  # 获取 Git 状态详情
```

### 6.3 Gateway 注册

```
POST /api/gateways/register     # Gateway 启动时注册
```

---

## 7. 需要修改的文件

### 7.1 后端 Rust

| 文件 | 修改内容 |
|------|----------|
| `crates/vk-core/src/lib.rs` | 导出新模块 |
| `crates/vk-core/src/project.rs` | 新建: Project 模型 |
| `crates/vk-core/src/task.rs` | 添加 Git 相关字段 |
| `crates/git-worktree/src/worktree.rs` | 添加 merge, push 方法 |
| `crates/api-server/src/routes/project.rs` | 新建: 项目 API |
| `crates/api-server/src/routes/task.rs` | 更新创建任务逻辑 |
| `crates/api-server/src/routes/gateway.rs` | 更新注册逻辑 |

### 7.2 Gateway (TypeScript)

| 文件 | 修改内容 |
|------|----------|
| `services/agent-gateway/src/index.ts` | 添加项目路径检测和注册 |
| `services/agent-gateway/src/executor.ts` | 添加 worktree 创建、commit、merge、push |
| `services/agent-gateway/src/git.ts` | 新建: Git 操作封装 |

### 7.3 前端

| 文件 | 修改内容 |
|------|----------|
| `packages/client/src/components/task/CreateTaskModal.tsx` | 添加项目选择、分支名、合并策略 UI |
| `packages/client/src/components/sidebar/` | 添加项目列表显示 |
| `packages/client/src/hooks/useProjects.ts` | 新建: 项目数据 hook |

---

## 8. 实现计划

### Phase 1: 数据模型 (预计 2-3 小时)
- [ ] 创建 Project 模型
- [ ] 更新 Task 模型
- [ ] 数据库迁移

### Phase 2: Gateway 注册 (预计 2-3 小时)
- [ ] Gateway 项目检测
- [ ] 注册 API
- [ ] 前端项目列表

### Phase 3: Git 操作集成 (预计 4-6 小时)
- [ ] Worktree 创建/删除
- [ ] 自动 commit
- [ ] Merge 逻辑
- [ ] Push 到远程

### Phase 4: 前端 UI (预计 3-4 小时)
- [ ] 创建任务 Modal 更新
- [ ] Git 状态显示
- [ ] 合并/清理按钮

---

## 9. 待确定事项

- [ ] API 接口详细的请求/响应格式
- [ ] 错误处理策略
- [ ] 前端项目切换 UI

---

## 附录: 现有 git-worktree 实现

`crates/git-worktree/src/worktree.rs` 已有功能：
- `create()` - 创建 worktree
- `remove()` - 删除 worktree  
- `list()` - 列出所有 worktree
- `commit_all()` - 提交所有更改
- `get_diff()` - 获取差异
- `has_uncommitted_changes()` - 检查未提交更改

需要添加：
- `merge_to_main()` - 合并到主分支
- `push()` - 推送到远程
- `resolve_conflicts()` - AI 解决冲突的接口（如果需要）
