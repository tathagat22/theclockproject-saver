# Windows screensaver registration script
# Run as Administrator after building the Tauri app
#
# On Windows, a .scr file is just a renamed .exe that responds to:
#   /s   → start screensaver
#   /c   → show config dialog
#   /p   → preview in handle
#
# Steps:
# 1. Build: npm run build  (produces src-tauri/target/release/theclockproject-saver.exe)
# 2. Copy .exe → .scr and register
# 3. Select from Windows Settings → Personalization → Screen saver

param(
    [string]$BuildDir = "$PSScriptRoot\..\..\src-tauri\target\release"
)

$exe = Join-Path $BuildDir "theclockproject-saver.exe"
$scr = "$env:SystemRoot\System32\ClockProjectSaver.scr"

if (-not (Test-Path $exe)) {
    Write-Error "Build the Tauri app first: npm run build"
    exit 1
}

Copy-Item -Force $exe $scr
Write-Host "Installed to $scr"
Write-Host "Open Settings > Personalization > Lock screen > Screen saver to activate."
