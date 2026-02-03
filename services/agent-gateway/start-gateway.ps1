$env:GATEWAY_SERVER_URL = "ws://localhost:8081"
$env:GATEWAY_HOST_ID = "local-gateway-1"
$env:GATEWAY_CWD = "C:\Users\25911\gateway-test-repo"

# 重要：清除 attach 模式，使用独立的 opencode 实例
Remove-Item Env:OPENCODE_ATTACH_SERVER -ErrorAction SilentlyContinue

Write-Host "Starting Gateway with:"
Write-Host "  Server: $env:GATEWAY_SERVER_URL"
Write-Host "  Host ID: $env:GATEWAY_HOST_ID"
Write-Host "  CWD: $env:GATEWAY_CWD"
Write-Host "  Mode: Independent (no attach)"

npx tsx src/index.ts
