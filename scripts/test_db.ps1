param(
  [ValidateSet('start','stop','status','logs')]
  [string]$cmd = 'start'
)

$ErrorActionPreference = 'Stop'
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir
if (-not $projectRoot) { $projectRoot = (Get-Location).Path }
Set-Location $projectRoot

$composeFile = Join-Path $projectRoot 'docker-compose.test-db.yml'
$serviceName = 'test-db'
$port = 55432

function Invoke-Compose {
  param([string[]]$ComposeArgs)
  if (Get-Command podman -ErrorAction SilentlyContinue) {
    & podman compose @('-f', $composeFile) @ComposeArgs
  } elseif (Get-Command podman-compose -ErrorAction SilentlyContinue) {
    & podman-compose @('-f', $composeFile) @ComposeArgs
  } else {
    throw "Podman Compose not found. Please install Podman Desktop or podman-compose."
  }
}

function Show-ConnectionInfo {
  Write-Host "Test PostgreSQL connection details:" -ForegroundColor Cyan
  Write-Host "  host=localhost port=$port user=timekeeper_test password=timekeeper_test db=timekeeper_test" -ForegroundColor Gray
  Write-Host "  DATABASE_URL=postgres://timekeeper_test:timekeeper_test@localhost:$port/timekeeper_test" -ForegroundColor Gray
}

function Start-TestDb {
  Write-Host "Starting test PostgreSQL on port $port..." -ForegroundColor Cyan
  Invoke-Compose @('up','-d',$serviceName) | Out-Null
  Show-ConnectionInfo
}

function Stop-TestDb {
  Write-Host "Stopping test PostgreSQL container..." -ForegroundColor Cyan
  try {
    Invoke-Compose @('stop',$serviceName) | Out-Null
    Write-Host "Stopped." -ForegroundColor Green
  } catch {
    Write-Host "Failed to stop test db: $_" -ForegroundColor Yellow
  }
}

function Status-TestDb {
  Write-Host "Test PostgreSQL status:" -ForegroundColor Cyan
  try {
    Invoke-Compose @('ps',$serviceName)
    Show-ConnectionInfo
  } catch {
    Write-Host "Unable to query status: $_" -ForegroundColor Red
  }
}

function Logs-TestDb {
  Write-Host "Tailing test PostgreSQL logs (last 200 lines)..." -ForegroundColor Cyan
  try {
    Invoke-Compose @('logs','-f','--tail=200',$serviceName)
  } catch {
    Write-Host "Unable to show logs: $_" -ForegroundColor Red
  }
}

switch ($cmd) {
  'start'  { Start-TestDb }
  'stop'   { Stop-TestDb }
  'status' { Status-TestDb }
  'logs'   { Logs-TestDb }
}
