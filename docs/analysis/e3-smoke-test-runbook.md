# E3/E4/E5 本地试运行手册（可直接复制执行）

适用工作区：`C:\Users\25911\Desktop\remote-worktrees\orch-e3-execution-api`

目标：一次性验证你当前这版的核心能力是否可用：
- Execution Orchestrator API（`/api/v1/executions`）
- Ops Console API（`/api/v1/ops/*`）
- Memory Enhanced（去重 + 新参数）
- 兼容层（`/api/tasks/*` deprecation header）

## 1. 启动服务
在终端 A 执行：

```powershell
cd C:\Users\25911\Desktop\remote-worktrees\orch-e3-execution-api
pnpm install
pnpm --dir services/agent-gateway install --ignore-workspace
pnpm run dev:local
```

预期：
- Rust API 启动：`REST API listening on 0.0.0.0:8081`
- Gateway 注册成功：`Registered successfully`
- Client 启动（默认 Vite 5173）

注意：
- 你之前遇到的 `@opencode-vibe/protocol` 入口问题已在脚本里通过 `predev` 预构建修复。
- 启动初期网关短暂 `ECONNREFUSED` 属于 Rust API 尚未起来，后续会自动恢复。

## 2. 一键 Smoke 测试（终端 B）
保持终端 A 不关，在终端 B 执行以下脚本。

最省事方式（推荐）：

```powershell
cd C:\Users\25911\Desktop\remote-worktrees\orch-e3-execution-api
powershell -ExecutionPolicy Bypass -File .\scripts\smoke-e3.ps1
```

你也可以手动执行下面的分步脚本：

```powershell
$ErrorActionPreference = "Stop"
$api = "http://127.0.0.1:8081"

function J($o) { $o | ConvertTo-Json -Depth 20 }

Write-Host "== [1] health =="
$health = Invoke-RestMethod "$api/health"
$health | J | Write-Host
if ($health.status -ne "ok") { throw "health status is not ok" }

Write-Host "`n== [2] hosts =="
$hosts = Invoke-RestMethod "$api/api/hosts"
if (-not $hosts -or $hosts.Count -eq 0) {
  throw "No hosts connected. Check gateway logs in terminal A."
}
$selectedHost = $hosts[0]
$hostId = $selectedHost.hostId
Write-Host "Using hostId: $hostId"

Write-Host "`n== [3] create workspace =="
$wsName = "e3-smoke-" + (Get-Date -Format "MMdd-HHmmss")
$wsReq = @{
  name = $wsName
  hostId = $hostId
  rootPath = $health.repoPath
}
$ws = Invoke-RestMethod -Method Post -Uri "$api/api/workspaces" -ContentType "application/json" -Body (J $wsReq)
$wsId = $ws.id
Write-Host "workspaceId: $wsId"

Write-Host "`n== [4] create project =="
$projPath = Join-Path $health.repoPath (".tmp-e3-smoke-" + (Get-Date -Format "yyyyMMdd-HHmmss"))
$projReq = @{
  name = "e3-smoke-project"
  localPath = $projPath
  defaultBranch = "main"
}
$proj = Invoke-RestMethod -Method Post -Uri "$api/api/workspaces/$wsId/projects" -ContentType "application/json" -Body (J $projReq)
$projectId = $proj.id
Write-Host "projectId: $projectId"

Write-Host "`n== [5] create task =="
$taskReq = @{
  title = "E3 smoke task " + (Get-Date -Format "HH:mm:ss")
  description = "Smoke test for orchestrator v1"
  projectId = $projectId
  agentType = "opencode"
  baseBranch = "main"
}
$task = Invoke-RestMethod -Method Post -Uri "$api/api/tasks" -ContentType "application/json" -Body (J $taskReq)
$taskId = $task.id
Write-Host "taskId: $taskId"

Write-Host "`n== [6] create execution (/api/v1/executions) =="
$traceId = "trace-e3-smoke-" + ([guid]::NewGuid().ToString("N").Substring(0, 8))
$execReq = @{
  taskId = $taskId
  agentType = "opencode"
  traceId = $traceId
  orgId = "org-smoke"
}
$exec = Invoke-RestMethod -Method Post -Uri "$api/api/v1/executions" -ContentType "application/json" -Body (J $execReq)
$executionId = $exec.executionId
Write-Host "executionId: $executionId"

Write-Host "`n== [7] poll execution detail =="
$detail = $null
for ($i = 0; $i -lt 30; $i++) {
  Start-Sleep -Seconds 2
  $detail = Invoke-RestMethod "$api/api/v1/executions/$executionId"
  Write-Host "status[$i] = $($detail.status)"
  if ($detail.status -eq "running") { break }
  if ($detail.status -in @("completed", "failed", "cancelled")) { break }
}
if (-not $detail) { throw "execution detail not found" }

Write-Host "`n== [8] send input (idempotent-safe) =="
$inputRes = Invoke-RestMethod -Method Post -Uri "$api/api/v1/executions/$executionId/input" -ContentType "application/json" -Body (J @{ content = "请回报当前进度" })
$inputRes | J | Write-Host

Write-Host "`n== [9] list events =="
$events = Invoke-RestMethod "$api/api/v1/executions/$executionId/events?limit=30"
Write-Host "events.count = $($events.events.Count), hasMore = $($events.hasMore)"

Write-Host "`n== [10] stop execution =="
$stopRes = Invoke-RestMethod -Method Post -Uri "$api/api/v1/executions/$executionId/stop"
$stopRes | J | Write-Host

Write-Host "`n== [11] ops summary + audit =="
$summary = Invoke-RestMethod "$api/api/v1/ops/summary"
$audit = Invoke-RestMethod "$api/api/v1/ops/audit?executionId=$executionId&limit=20"
Write-Host "ops.executions.total = $($summary.executions.total)"
Write-Host "audit.items = $($audit.items.Count)"

Write-Host "`n== [12] memory enhanced (settings + dedupe) =="
$settingsBefore = Invoke-RestMethod "$api/api/memory/settings"
$patchReq = @{
  patch = @{
    dedupeEnabled = $true
    recencyHalfLifeHours = 48
    hitCountWeight = 0.25
    pinnedBoost = 1.5
  }
}
$settingsAfter = Invoke-RestMethod -Method Patch -Uri "$api/api/memory/settings" -ContentType "application/json" -Body (J $patchReq)

$memReq = @{
  hostId = $hostId
  projectId = $projectId
  scope = "project"
  kind = "fact"
  content = "E3 smoke memory item"
  tags = @("e3", "smoke")
}
$m1 = Invoke-RestMethod -Method Post -Uri "$api/api/memory/items" -ContentType "application/json" -Body (J $memReq)
$m2 = Invoke-RestMethod -Method Post -Uri "$api/api/memory/items" -ContentType "application/json" -Body (J $memReq)
Write-Host "memory dedupe sameId = $($m1.id -eq $m2.id)"

Write-Host "`n== [13] legacy compatibility headers =="
$legacy = Invoke-WebRequest "$api/api/tasks" -UseBasicParsing
Write-Host "Deprecation: $($legacy.Headers['Deprecation'])"
Write-Host "Link: $($legacy.Headers['Link'])"

Write-Host "`nSMOKE TEST DONE"
```

## 3. 关键通过标准（你只看这 8 条）
1. `/health` 返回 `status=ok`，且有 `featureFlags.orchestratorV1=true`。  
2. `/api/hosts` 至少 1 个 host。  
3. `POST /api/v1/executions` 返回 `executionId`。  
4. `GET /api/v1/executions/{id}/events` 能看到事件（`events.count >= 1`）。  
5. `POST /api/v1/executions/{id}/input`、`/stop` 返回成功结构（`accepted` 可能 true/false，均为幂等设计）。  
6. `/api/v1/ops/summary` 和 `/api/v1/ops/audit` 有数据。  
7. 连续两次创建同 content 的 memory item，`id` 相同（dedupe 生效）。  
8. 旧接口 `/api/tasks` 响应头包含：`Deprecation: true` 与 successor `Link`。  

## 4. 常见失败与处理
1. `404 /api/v1/executions`  
原因：`FEATURE_ORCHESTRATOR_V1` 被关了。  
处理：启动前设置 `FEATURE_ORCHESTRATOR_V1=true`。  

2. 创建 execution 报 host 不在线  
原因：Gateway 未注册。  
处理：看终端 A 是否有 `Registered successfully`。  

3. `POST /api/tasks` 报 `Project is required`  
原因：漏传 `projectId`。  
处理：必须先建 workspace/project，再建 task。  

4. memory 接口报冲突（stores disabled）  
原因：`gatewayStoreEnabled=false` 且 `rustStoreEnabled=false`。  
处理：至少开一个存储后再测。  
