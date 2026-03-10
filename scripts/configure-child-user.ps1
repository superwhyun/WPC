param(
    [Parameter(Mandatory = $true)]
    [string]$ChildUser,

    [Parameter(Mandatory = $true)]
    [string]$Pin,

    [Parameter(Mandatory = $true)]
    [string[]]$AllowedLogins,

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

$sid = (New-Object System.Security.Principal.NTAccount($ChildUser)).Translate([System.Security.Principal.SecurityIdentifier]).Value
$startupDir = Join-Path "C:\Users\$ChildUser\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup" ""
New-Item -ItemType Directory -Force -Path $startupDir | Out-Null

$startupCmd = Join-Path $startupDir "WinParentalControlAgent.cmd"
$startupContent = @"
@echo off
start "" /min "$agentExe"
"@
Set-Content -Path $startupCmd -Value $startupContent -Encoding ASCII

$args = @(
    "--init-config",
    "--protected-user-sid", $sid,
    "--pin", $Pin
)

foreach ($login in $AllowedLogins) {
    $args += @("--allowed-login", $login)
}

& $serviceExe @args
Restart-Service -Name $ServiceName

Write-Host "Protected child user configured: $ChildUser ($sid)"
Write-Host "Allowed parent logins: $($AllowedLogins -join ', ')"
