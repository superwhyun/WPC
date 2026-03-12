param(
    [string]$BuildDir = (Join-Path $PSScriptRoot "..\target\release"),
    [string]$InstallDir = (Join-Path $env:ProgramFiles "WinParentalControl"),
    [string]$ServiceName = "WinParentalControlService"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $BuildDir)) {
    throw "Build output not found: $BuildDir"
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item (Join-Path $BuildDir "winpc-service.exe") $InstallDir -Force

$serviceExe = Join-Path $InstallDir "winpc-service.exe"
$binPath = "`"$serviceExe`""

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
    if ($existing.Status -ne "Stopped") {
        Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
    }
    sc.exe delete $ServiceName | Out-Null
    Start-Sleep -Seconds 2
}

sc.exe create $ServiceName binPath= $binPath start= auto DisplayName= "WinParentalControl Service" | Out-Null
sc.exe description $ServiceName "Parental control service for lock, unlock and session timing." | Out-Null
sc.exe failure $ServiceName reset= 86400 actions= restart/5000 | Out-Null

Start-Service -Name $ServiceName
Write-Host "Service installed and started: $ServiceName"
