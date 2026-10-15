# Run from the repository root or from VS Code's integrated PowerShell terminal:
#   .\scripts\dev.ps1
[CmdletBinding()]
param(
    [switch]$SkipInstall
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$backendPath = Join-Path $root 'backend'
$frontendPath = Join-Path $root 'frontend'
$composePath = Join-Path $root 'compose.dev.yml'
$backendEnv = Join-Path $backendPath '.env'
$frontendEnv = Join-Path $frontendPath '.env'

function Test-TcpPort([string]$HostName, [int]$Port, [int]$TimeoutMs = 800) {
    $client = [System.Net.Sockets.TcpClient]::new()
    try {
        $pending = $client.BeginConnect($HostName, $Port, $null, $null)
        if (-not $pending.AsyncWaitHandle.WaitOne($TimeoutMs, $false)) {
            return $false
        }
        $client.EndConnect($pending)
        return $client.Connected
    }
    catch {
        return $false
    }
    finally {
        $client.Dispose()
    }
}

function Test-ApiReady {
    try {
        $response = Invoke-RestMethod -Uri 'http://127.0.0.1:8080/health/ready' -TimeoutSec 2
        return ($response.status -eq 'ready' -and $response.database -eq 'ok')
    }
    catch {
        return $false
    }
}

function Get-ApiLive {
    try {
        $null = Invoke-RestMethod -Uri 'http://127.0.0.1:8080/health/live' -TimeoutSec 2
        return $true
    }
    catch {
        return $false
    }
}

foreach ($command in @('node', 'npm', 'cargo')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Required command '$command' was not found. Install Node.js 22.12+ and the stable Rust toolchain, then reopen VS Code."
    }
}

$nodeText = (& node --version).Trim()
$nodeVersion = [version]$nodeText.TrimStart('v')
if ($nodeVersion -lt [version]'22.12.0') {
    throw "Node.js 22.12+ is required; this terminal reports $nodeText."
}
$npmText = (& npm --version).Trim()
$npmVersion = [version]$npmText
if ($npmVersion.Major -lt 11) {
    Write-Warning "npm 11 is recommended for this project (detected $npmText). If dependency installation fails, run: npm install --global npm@11"
}

if (-not (Test-Path $backendEnv)) {
    Copy-Item (Join-Path $backendPath '.env.example') $backendEnv
    Write-Host 'Created backend/.env from the development template. Review its values before using the app.' -ForegroundColor Yellow
}
if (-not (Test-Path $frontendEnv)) {
    Copy-Item (Join-Path $frontendPath '.env.example') $frontendEnv
    Write-Host 'Created frontend/.env from the development template.' -ForegroundColor Yellow
}

if (-not (Test-TcpPort '127.0.0.1' 27017)) {
    if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        throw "MongoDB is not reachable at 127.0.0.1:27017 and Docker is not installed. Start MongoDB locally or install Docker Desktop, then run this script again. The Rust API cannot start without a reachable database."
    }

    # The Docker CLI can be installed while the Docker Desktop Linux engine is stopped.
    # Check the daemon before trying docker compose so the failure is clear and recoverable.
    & docker info --format '{{.ServerVersion}}' 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) {
        $dockerDesktopCandidates = @(
            (Join-Path $env:ProgramFiles 'Docker\Docker\Docker Desktop.exe'),
            (Join-Path $env:LOCALAPPDATA 'Docker\Docker Desktop.exe')
        ) | Where-Object { $_ -and (Test-Path $_) }

        if ($dockerDesktopCandidates.Count -gt 0) {
            $dockerDesktopPath = $dockerDesktopCandidates[0]
            Write-Host 'Docker Desktop is installed but its engine is not responding. Starting Docker Desktop...' -ForegroundColor Yellow
            try {
                Start-Process -FilePath $dockerDesktopPath | Out-Null
            }
            catch {
                throw "Could not launch Docker Desktop at '$dockerDesktopPath'. Open Docker Desktop manually, wait for its engine to start, then run this script again."
            }

            $dockerDeadline = (Get-Date).AddMinutes(2)
            while ((Get-Date) -lt $dockerDeadline) {
                & docker info --format '{{.ServerVersion}}' 2>$null | Out-Null
                if ($LASTEXITCODE -eq 0) { break }
                Start-Sleep -Seconds 3
            }

            & docker info --format '{{.ServerVersion}}' 2>$null | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw 'Docker Desktop did not start its engine within 2 minutes. Open Docker Desktop, wait until it says the engine is running, then check docker info and retry .\scripts\dev.ps1.'
            }
        }
        else {
            throw "Docker CLI is installed, but the Docker Desktop engine is unavailable and the Desktop app was not found in the standard install paths. Start Docker Desktop manually or install MongoDB Community Server and start it on 127.0.0.1:27017. Verify Docker with docker info, then rerun this script."
        }
    }

    Write-Host 'MongoDB is not running on port 27017; starting the repository development MongoDB container...' -ForegroundColor Cyan
    & docker compose -f $composePath up -d mongo
    if ($LASTEXITCODE -ne 0) {
        throw 'Docker is reachable but could not start MongoDB. Check docker compose -f compose.dev.yml logs mongo and make sure Docker Desktop is using Linux containers.'
    }
    $mongoDeadline = (Get-Date).AddSeconds(60)
    while ((Get-Date) -lt $mongoDeadline -and -not (Test-TcpPort '127.0.0.1' 27017)) {
        Start-Sleep -Seconds 2
    }
    if (-not (Test-TcpPort '127.0.0.1' 27017)) {
        throw 'MongoDB did not open port 27017 within 60 seconds. Check Docker Desktop and run: docker compose -f compose.dev.yml logs mongo'
    }
}

$shell = (Get-Command pwsh.exe -ErrorAction SilentlyContinue)
if (-not $shell) {
    $shell = Get-Command powershell.exe -ErrorAction SilentlyContinue
}
if (-not $shell) {
    throw 'Could not find PowerShell to open service terminals.'
}

if (-not (Test-ApiReady)) {
    if (Test-TcpPort '127.0.0.1' 8080) {
        if (Get-ApiLive) {
            throw 'Something is listening on port 8080, but /health/ready does not report database=ok. Inspect that backend terminal and backend/.env; do not start a second API process.'
        }
        throw 'Port 8080 is occupied by a service that does not expose the Newage /health/live endpoint. Stop it or change the configured port before continuing.'
    }

    Write-Host 'Starting the Rust API in a new terminal...' -ForegroundColor Cyan
    Start-Process -FilePath $shell.Source -WorkingDirectory $backendPath -ArgumentList @('-NoExit', '-Command', 'cargo run')

    $apiDeadline = (Get-Date).AddMinutes(2)
    while ((Get-Date) -lt $apiDeadline -and -not (Test-ApiReady)) {
        Start-Sleep -Seconds 3
    }
    if (-not (Test-ApiReady)) {
        throw "The Rust API did not become ready within 2 minutes. Read the backend terminal output. Check backend/.env, MongoDB, and then test http://127.0.0.1:8080/health/ready."
    }
}
else {
    Write-Host 'Rust API and MongoDB are already ready.' -ForegroundColor Green
}

if (-not $SkipInstall -and -not (Test-Path (Join-Path $frontendPath 'node_modules'))) {
    Write-Host 'Installing frontend dependencies...' -ForegroundColor Cyan
    Push-Location $frontendPath
    try {
        & npm install
        if ($LASTEXITCODE -ne 0) {
            throw 'npm install failed. Review the npm output; if npm is version 10, update to npm 11 and retry.'
        }
    }
    finally {
        Pop-Location
    }
}

if (Test-TcpPort '127.0.0.1' 3000) {
    Write-Host 'Port 3000 is already in use. Open http://localhost:3000 and inspect the existing frontend terminal.' -ForegroundColor Yellow
}
else {
    Write-Host 'Starting the Nuxt frontend in a new terminal...' -ForegroundColor Cyan
    Start-Process -FilePath $shell.Source -WorkingDirectory $frontendPath -ArgumentList @('-NoExit', '-Command', 'npm run dev')
}

Write-Host ''
Write-Host 'Newage development environment is ready to test:' -ForegroundColor Green
Write-Host '  Frontend:    http://localhost:3000'
Write-Host '  API liveness: http://127.0.0.1:8080/health/live'
Write-Host '  API + DB:     http://127.0.0.1:8080/health/ready'
Write-Host ''
Write-Host 'Test theme at /dashboard/site, people at /dashboard/users, and courses at /dashboard.'
Write-Host 'If a request fails, keep both terminal windows open and inspect the reported HTTP/API error.'
