$ErrorActionPreference = "Stop"

$api = "http://127.0.0.1:8081"

function To-JsonBody {
  param([Parameter(Mandatory = $true)]$InputObject)
  $InputObject | ConvertTo-Json -Depth 20
}

Write-Host "== [1/7] health =="
$health = Invoke-RestMethod "$api/health"
if ($health.status -ne "ok") {
  throw "health status is not ok"
}
Write-Host "health ok"

Write-Host ""
Write-Host "== [2/7] register user =="
$timestamp = Get-Date -Format "yyyyMMddHHmmss"
$email = "tenant-$timestamp@example.com"
$registerReq = @{
  email = $email
  password = "tenant-pass"
  name = "Tenant User"
  orgName = "Tenant Org $timestamp"
}
$register = Invoke-RestMethod -Method Post -Uri "$api/api/v1/auth/register" -ContentType "application/json" -Body (To-JsonBody $registerReq)
$token = $register.token
$orgId = $register.orgId
if (-not $token) { throw "register token missing" }
if (-not $orgId) { throw "register orgId missing" }
Write-Host "registered: user=$($register.userId), org=$orgId"

Write-Host ""
Write-Host "== [3/7] login user =="
$loginReq = @{
  email = $email
  password = "tenant-pass"
  orgId = $orgId
}
$login = Invoke-RestMethod -Method Post -Uri "$api/api/v1/auth/login" -ContentType "application/json" -Body (To-JsonBody $loginReq)
if (-not $login.token) { throw "login token missing" }
Write-Host "login ok"

Write-Host ""
Write-Host "== [4/7] me + orgs =="
$authHeader = @{ Authorization = "Bearer $token" }
$me = Invoke-RestMethod -Method Get -Uri "$api/api/v1/me" -Headers $authHeader
$orgs = Invoke-RestMethod -Method Get -Uri "$api/api/v1/orgs" -Headers $authHeader
Write-Host "me.orgId = $($me.orgId), me.role = $($me.role), orgs.count = $($orgs.Count)"

Write-Host ""
Write-Host "== [5/7] enroll host token =="
$hostId = "host-e2-smoke"
$enrollReq = @{ hostId = $hostId; expiresInHours = 24 }
$enroll = Invoke-RestMethod -Method Post -Uri "$api/api/v1/orgs/$orgId/hosts/enroll" -Headers $authHeader -ContentType "application/json" -Body (To-JsonBody $enrollReq)
if (-not $enroll.token) { throw "host enroll token missing" }
Write-Host "enroll ok, tokenVersion=$($enroll.tokenVersion)"

Write-Host ""
Write-Host "== [6/7] rotate host token =="
$rotateReq = @{ hostId = $hostId; expiresInHours = 24 }
$rotate = Invoke-RestMethod -Method Post -Uri "$api/api/v1/orgs/$orgId/hosts/rotate-token" -Headers $authHeader -ContentType "application/json" -Body (To-JsonBody $rotateReq)
if (-not $rotate.token) { throw "host rotate token missing" }
if ($rotate.tokenVersion -le $enroll.tokenVersion) { throw "tokenVersion did not increase" }
Write-Host "rotate ok, tokenVersion=$($rotate.tokenVersion)"

Write-Host ""
Write-Host "== [7/7] disable host =="
$disableReq = @{ hostId = $hostId }
$disable = Invoke-RestMethod -Method Post -Uri "$api/api/v1/orgs/$orgId/hosts/disable" -Headers $authHeader -ContentType "application/json" -Body (To-JsonBody $disableReq)
if ($disable.enabled -ne $false) { throw "host disable failed" }
Write-Host "disable ok"

Write-Host ""
Write-Host "SMOKE E1/E2 PASSED"
