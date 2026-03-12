param(
    [Parameter(Mandatory = $true)]
    [string]$Pin,

    [string]$InstallDir = (Join-Path $env:ProgramFiles "WinParentalControl"),
    [string]$ServiceName = "WinParentalControlService"
)

$ErrorActionPreference = "Stop"

$serviceExe = Join-Path $InstallDir "winpc-service.exe"
$agentExe = Join-Path $InstallDir "winpc-agent.exe"

if (-not (Test-Path $serviceExe)) {
    throw "Service executable not found: $serviceExe"
}

if (-not (Test-Path $agentExe)) {
    throw "Agent executable not found: $agentExe"
}

$startupDir = Join-Path $env:ProgramData "Microsoft\Windows\Start Menu\Programs\StartUp"
New-Item -ItemType Directory -Force -Path $startupDir | Out-Null

$startupCmd = Join-Path $startupDir "WinParentalControlAgent.cmd"
$startupContent = @"
@echo off
start "" /min "$agentExe"
"@
Set-Content -Path $startupCmd -Value $startupContent -Encoding ASCII

$args = @(
    "--init-config",
    "--all-users",
    "--pin", $Pin
)

& $serviceExe @args
Restart-Service -Name $ServiceName

Write-Host "Configured parental control for all interactive users."
