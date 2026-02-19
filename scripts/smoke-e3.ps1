$ErrorActionPreference = "Stop"

$api = "http://127.0.0.1:8081"

function To-JsonBody {
  param([Parameter(Mandatory = $true)]$InputObject)
  $InputObject | ConvertTo-Json -Depth 20
}

Write-Host "== [1/13] health =="
$health = Invoke-RestMethod "$api/health"
if ($health.status -ne "ok") {
  throw "health status is not ok"
}
if (-not $health.featureFlags.orchestratorV1) {
  throw "FEATURE_ORCHESTRATOR_V1 is disabled (orchestratorV1=false)"
}
Write-Host "health ok, repoPath: $($health.repoPath)"

Write-Host ""
Write-Host "== [2/13] hosts =="
$hosts = Invoke-RestMethod "$api/api/hosts"
if (-not $hosts -or $hosts.Count -eq 0) {
  throw "No hosts connected. Check gateway process logs."
}
$selectedHost = $hosts[0]
$hostId = $selectedHost.hostId
Write-Host "using hostId: $hostId"

Write-Host ""
Write-Host "== [3/13] create workspace =="
$wsName = "e3-smoke-" + (Get-Date -Format "MMdd-HHmmss")
$orgId = "org-smoke"
$wsReq = @{
  name     = $wsName
  orgId    = $orgId
  hostId   = $hostId
  rootPath = $health.repoPath
}
$ws = Invoke-RestMethod -Method Post -Uri "$api/api/workspaces" -ContentType "application/json" -Body (To-JsonBody $wsReq)
$wsId = $ws.id
Write-Host "workspaceId: $wsId"

Write-Host ""
Write-Host "== [4/13] create project =="
$projPath = Join-Path $health.repoPath (".tmp-e3-smoke-" + (Get-Date -Format "yyyyMMdd-HHmmss"))
$projReq = @{
  name          = "e3-smoke-project"
  localPath     = $projPath
  defaultBranch = "main"
}
$proj = Invoke-RestMethod -Method Post -Uri "$api/api/workspaces/$wsId/projects" -ContentType "application/json" -Body (To-JsonBody $projReq)
$projectId = $proj.id
Write-Host "projectId: $projectId"

Write-Host ""
Write-Host "== [5/13] create task =="
$taskReq = @{
  title       = "E3 smoke task " + (Get-Date -Format "HH:mm:ss")
  description = "Smoke test for orchestrator v1"
  projectId   = $projectId
  agentType   = "opencode"
  baseBranch  = "main"
}
$task = Invoke-RestMethod -Method Post -Uri "$api/api/tasks" -ContentType "application/json" -Body (To-JsonBody $taskReq)
$taskId = $task.id
Write-Host "taskId: $taskId"

Write-Host ""
Write-Host "== [6/13] create execution (v1) =="
$traceId = "trace-e3-smoke-" + ([guid]::NewGuid().ToString("N").Substring(0, 8))
$execReq = @{
  taskId    = $taskId
  agentType = "opencode"
  traceId   = $traceId
  orgId     = $orgId
}
$exec = Invoke-RestMethod -Method Post -Uri "$api/api/v1/executions" -ContentType "application/json" -Body (To-JsonBody $execReq)
$executionId = $exec.executionId
Write-Host "executionId: $executionId"

Write-Host ""
Write-Host "== [7/13] poll execution status =="
$detail = $null
for ($i = 0; $i -lt 30; $i++) {
  Start-Sleep -Seconds 2
  $detail = Invoke-RestMethod "$api/api/v1/executions/$executionId"
  Write-Host "status[$i] = $($detail.status)"
  if ($detail.status -eq "running") { break }
  if ($detail.status -in @("completed", "failed", "cancelled")) { break }
}
if (-not $detail) {
  throw "execution detail not found"
}

Write-Host ""
Write-Host "== [8/13] send input =="
$inputReq = @{ content = "Please report current progress." }
$inputRes = Invoke-RestMethod -Method Post -Uri "$api/api/v1/executions/$executionId/input" -ContentType "application/json" -Body (To-JsonBody $inputReq)
Write-Host "input accepted: $($inputRes.accepted), status: $($inputRes.status)"

Write-Host ""
Write-Host "== [9/13] list events =="
$events = Invoke-RestMethod "$api/api/v1/executions/$executionId/events?limit=30"
Write-Host "events.count = $($events.events.Count), hasMore = $($events.hasMore)"
if ($events.events.Count -lt 1) {
  throw "no execution events returned"
}

Write-Host ""
Write-Host "== [10/13] stop execution =="
$stopRes = Invoke-RestMethod -Method Post -Uri "$api/api/v1/executions/$executionId/stop"
Write-Host "stop accepted: $($stopRes.accepted), status: $($stopRes.status)"

Write-Host ""
Write-Host "== [11/13] ops summary + audit =="
$summary = Invoke-RestMethod "$api/api/v1/ops/summary"
$audit = Invoke-RestMethod "$api/api/v1/ops/audit?executionId=$executionId&limit=20"
Write-Host "ops.executions.total = $($summary.executions.total)"
Write-Host "audit.items = $($audit.items.Count)"
if ($audit.items.Count -lt 1) {
  throw "no audit items returned for execution"
}

Write-Host ""
Write-Host "== [12/13] memory enhanced checks =="
$null = Invoke-RestMethod "$api/api/memory/settings"
$patchReq = @{
  patch = @{
    dedupeEnabled       = $true
    recencyHalfLifeHours = 48
    hitCountWeight      = 0.25
    pinnedBoost         = 1.5
  }
}
$settingsAfter = Invoke-RestMethod -Method Patch -Uri "$api/api/memory/settings" -ContentType "application/json" -Body (To-JsonBody $patchReq)
Write-Host "memory settings patched, dedupeEnabled = $($settingsAfter.dedupeEnabled)"

$memReq = @{
  hostId    = $hostId
  projectId = $projectId
  scope     = "project"
  kind      = "fact"
  content   = "E3 smoke memory item"
  tags      = @("e3", "smoke")
}
$m1 = Invoke-RestMethod -Method Post -Uri "$api/api/memory/items" -ContentType "application/json" -Body (To-JsonBody $memReq)
$m2 = Invoke-RestMethod -Method Post -Uri "$api/api/memory/items" -ContentType "application/json" -Body (To-JsonBody $memReq)
$dedupeSameId = $m1.id -eq $m2.id
Write-Host "memory dedupe sameId = $dedupeSameId"
if (-not $dedupeSameId) {
  throw "memory dedupe check failed (ids are different)"
}

Write-Host ""
Write-Host "== [13/13] legacy compatibility headers =="
$legacy = Invoke-WebRequest "$api/api/tasks" -UseBasicParsing
$deprecation = $legacy.Headers["Deprecation"]
$link = $legacy.Headers["Link"]
Write-Host "Deprecation: $deprecation"
Write-Host "Link: $link"
if ($deprecation -ne "true") {
  throw "Deprecation header check failed"
}
if ($link -notmatch "/api/v1/executions") {
  throw "Link header check failed"
}

Write-Host ""
Write-Host "SMOKE TEST PASSED"
