param(
  [ValidateSet('build','start','stop','status','logs')]
  [string]$cmd = 'start',
  [int]$port = 8000,
  [switch]$release
)

$ErrorActionPreference = 'Stop'
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir
Set-Location $projectRoot

$pidFile = Join-Path $projectRoot '.frontend.pid'
$logFile = Join-Path $projectRoot 'frontend\frontend-dev.log'

function Build-Frontend {
  Set-Location (Join-Path $projectRoot 'frontend')
  $mode = if ($release) { '--release' } else { '--dev' }
  if (-not (Get-Command wasm-pack -ErrorAction SilentlyContinue)) {
    Write-Host "wasm-pack is not installed. Please run: cargo install wasm-pack" -ForegroundColor Red
    throw "wasm-pack missing"
  }
  & wasm-pack build --target web --out-dir pkg $mode
}

function Start-Frontend {
  if (Test-Path $pidFile) {
    Write-Host "PID file exists. Use 'stop' first if process is stale." -ForegroundColor Yellow
  }
  Build-Frontend
  $script = "Set-Location frontend; python -m http.server $port *>&1 | Tee-Object -FilePath frontend-dev.log"
  $p = Start-Process -FilePath 'pwsh' -ArgumentList @('-NoLogo','-NoProfile','-Command', $script) -PassThru -WindowStyle Minimized
  $p.Id | Out-File -FilePath $pidFile -Encoding ascii -Force
  Write-Host "Frontend started. PID=$($p.Id). http://localhost:$port Logs: $logFile" -ForegroundColor Green
}

function Stop-Frontend {
  if (!(Test-Path $pidFile)) { Write-Host "No PID file; nothing to stop" -ForegroundColor Yellow; return }
  $pid = Get-Content $pidFile | Select-Object -First 1
  if ($pid) {
    try { Stop-Process -Id [int]$pid -Force; Write-Host "Stopped frontend (PID=$pid)" -ForegroundColor Green }
    catch { Write-Host "Failed to stop PID $pid: $_" -ForegroundColor Red }
  }
  Remove-Item $pidFile -ErrorAction SilentlyContinue
}

function Status-Frontend {
  if (!(Test-Path $pidFile)) { Write-Host "Status: not running"; return }
  $pid = Get-Content $pidFile | Select-Object -First 1
  $proc = Get-Process -Id ([int]$pid) -ErrorAction SilentlyContinue
  if ($proc) { Write-Host "Status: running (PID=$pid)" -ForegroundColor Green }
  else { Write-Host "Status: stale PID file ($pid)" -ForegroundColor Yellow }
}

switch ($cmd) {
  'build'  { Build-Frontend }
  'start'  { Start-Frontend }
  'stop'   { Stop-Frontend }
  'status' { Status-Frontend }
  'logs'   { if (Test-Path $logFile) { Get-Content $logFile -Tail 200 -Wait } else { Write-Host "No log file yet: $logFile" -ForegroundColor Yellow } }
}

