# Run this from the repo root (where package.json lives) before every
# `tauri build` / `tauri dev`. It does three things:
#   1. Builds the Next.js app in standalone mode.
#   2. Assembles the standalone output + static assets into
#      src-tauri/resources/server (this is what tauri.conf.json's
#      "bundle.resources" copies into the installed app).
#   3. Copies a portable Node.exe into src-tauri/binaries, renamed to the
#      exact filename Tauri's sidecar convention requires.
#
# Prerequisite (one-time): download the official Windows x64 Node build
# (the .zip, NOT the installer) from https://nodejs.org/en/download and
# extract it somewhere, e.g. C:\node-portable\node.exe. Set the path below
# to match, or pass it as -NodeExePath.

param(
    [string]$NodeExePath = "C:\node-portable\node.exe"
)

$ErrorActionPreference = "Stop"

Write-Host "==> Building Next.js (standalone output)..."
npm run build

$resourceDir = "src-tauri\resources\server"
if (Test-Path $resourceDir) {
    Remove-Item -Recurse -Force $resourceDir
}
New-Item -ItemType Directory -Force -Path $resourceDir | Out-Null

Write-Host "==> Copying standalone server output..."
Copy-Item -Recurse -Force ".next\standalone\*" $resourceDir

Write-Host "==> Copying static assets (not included in standalone output by default)..."
New-Item -ItemType Directory -Force -Path "$resourceDir\.next\static" | Out-Null
Copy-Item -Recurse -Force ".next\static\*" "$resourceDir\.next\static"

if (Test-Path "public") {
    Write-Host "==> Copying public/ assets..."
    Copy-Item -Recurse -Force "public" "$resourceDir\public"
}

$binDir = "src-tauri\binaries"
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

$targetTriple = "x86_64-pc-windows-msvc"
$sidecarName = "server-$targetTriple.exe"

if (-not (Test-Path $NodeExePath)) {
    $systemNode = (Get-Command node -ErrorAction SilentlyContinue).Source
    if ($systemNode -and (Test-Path $systemNode)) {
        Write-Host "==> Portable Node not found at '$NodeExePath', using system Node: $systemNode"
        $NodeExePath = $systemNode
    } else {
        Write-Error "Node binary not found at $NodeExePath. Download the Windows x64 .zip from https://nodejs.org/en/download, extract it, and pass -NodeExePath, or ensure node is in PATH."
    }
}

Write-Host "==> Staging Node runtime as sidecar ($sidecarName)..."
Copy-Item -Force $NodeExePath "$binDir\$sidecarName"

Write-Host "==> Done. Resource size:"
$size = (Get-ChildItem -Recurse $resourceDir | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host ("    {0:N1} MB in {1}" -f $size, $resourceDir)
Write-Host "==> Sidecar binary:"
Get-Item "$binDir\$sidecarName" | Select-Object Name, @{N="MB";E={[math]::Round($_.Length/1MB,1)}}
