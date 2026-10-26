# Windows + Linux build egy lépésben (Windowson futtasd, a Linux rész WSL-ben fordul).
#   powershell -ExecutionPolicy Bypass -File scripts\build-all.ps1
param(
    [string]$Distro = "Debian"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# A Rust ide települ; a rövid target mappa a Windows 260 karakteres útvonal-korlátja miatt kell.
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\kripto-ado-target" }

Write-Host "==> Windows build" -ForegroundColor Cyan
npm run tauri build -- --bundles nsis
if ($LASTEXITCODE -ne 0) { throw "Windows build sikertelen" }
$win = Join-Path $root "telepito\windows"
New-Item -ItemType Directory -Force $win | Out-Null
Remove-Item "$win\*" -Force -ErrorAction SilentlyContinue
Copy-Item "$env:CARGO_TARGET_DIR\release\bundle\nsis\*setup.exe" $win
Copy-Item "$env:CARGO_TARGET_DIR\release\kripto-ado.exe" $win

Write-Host "==> Linux build ($Distro WSL)" -ForegroundColor Cyan
$wslRoot = (wsl -d $Distro -- wslpath -a ($root -replace '\\', '/')).Trim()
wsl -d $Distro -- bash "$wslRoot/scripts/build-linux.sh" "$wslRoot"
if ($LASTEXITCODE -ne 0) { throw "Linux build sikertelen" }

Write-Host "==> Kész" -ForegroundColor Green
Get-ChildItem -Recurse (Join-Path $root "telepito") -File | Select-Object FullName, Length
