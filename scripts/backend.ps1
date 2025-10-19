param(
  [ValidateSet('start','stop','status','logs')]
  [string]$cmd = 'start'
)

$ErrorActionPreference = 'Stop'
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir
if (-not $projectRoot) { $projectRoot = (Get-Location).Path }
Set-Location $projectRoot

# Helper: choose Docker Compose v2 (`docker compose`) or legacy (`docker-compose`)
function Invoke-Compose {
  param([string[]]$ComposeArgs)
  if (Get-Command docker -ErrorAction SilentlyContinue) {
    & docker compose @ComposeArgs
  } elseif (Get-Command docker-compose -ErrorAction SilentlyContinue) {
    & docker-compose @ComposeArgs
  } else {
    throw "Docker Compose not found. Please install Docker Desktop or docker-compose."
  }
}

function Start-Backend {
  # Start only the backend service in detached mode
  Write-Host "Starting backend via Docker Compose..." -ForegroundColor Cyan
  Invoke-Compose @('up','-d','backend') | Out-Null
  # Remove stale PID file from previous non-Docker runner, if any
  $pidFile = Join-Path $projectRoot '.backend.pid'
  if (Test-Path $pidFile) { Remove-Item $pidFile -ErrorAction SilentlyContinue }
  Write-Host "Backend container started (service: backend)." -ForegroundColor Green
}

function Stop-Backend {
  Write-Host "Stopping backend container..." -ForegroundColor Cyan
  try {
    Invoke-Compose @('stop','backend') | Out-Null
    Write-Host "Backend container stopped." -ForegroundColor Green
  } catch {
    Write-Host "Failed to stop backend container: $_" -ForegroundColor Yellow
  }
}

function Status-Backend {
  Write-Host "Backend status (docker compose ps backend):" -ForegroundColor Cyan
  try {
    Invoke-Compose @('ps','backend')
  } catch {
    Write-Host "Unable to query status: $_" -ForegroundColor Red
  }
}

function Logs-Backend {
  Write-Host "Tailing backend logs (last 200 lines)..." -ForegroundColor Cyan
  try {
    Invoke-Compose @('logs','-f','--tail=200','backend')
  } catch {
    Write-Host "Unable to show logs: $_" -ForegroundColor Red
  }
}

switch ($cmd) {
  'start'  { Start-Backend }
  'stop'   { Stop-Backend }
  'status' { Status-Backend }
  'logs'   { Logs-Backend }
}
