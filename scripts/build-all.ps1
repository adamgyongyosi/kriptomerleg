# Windows + Linux build egy lépésben (Windowson futtasd, a Linux rész WSL-ben fordul).
#   powershell -ExecutionPolicy Bypass -File scripts\build-all.ps1
# A kimenetek a release-ével azonos nevűek: telepito\windows és telepito\linux.
param(
    [string]$Distro = "Debian"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# A Rust ide települ; a rövid target mappa a Windows 260 karakteres útvonal-korlátja miatt kell.
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\kripto-ado-target" }
$version = (Get-Content package.json -Raw | ConvertFrom-Json).version

# Az éppen futó app vagy a víruskereső egy pillanatra fogva tarthatja a fájlt.
function Copy-WithRetry([string]$From, [string]$To) {
    for ($i = 1; $i -le 10; $i++) {
        try {
            Copy-Item $From $To -Force
            return
        } catch {
            if ($i -eq 10) { throw "Nem sikerült másolni ($To): $_. Fut még a Kriptomérleg? Zárd be, és próbáld újra." }
            Start-Sleep -Seconds 2
        }
    }
}

Write-Host "==> Windows build ($version)" -ForegroundColor Cyan
npm run tauri build -- --bundles nsis
if ($LASTEXITCODE -ne 0) { throw "Windows build sikertelen" }
$win = Join-Path $root "telepito\windows"
New-Item -ItemType Directory -Force $win | Out-Null
Get-ChildItem $win -File | Remove-Item -Force -ErrorAction SilentlyContinue
$setup = Get-ChildItem "$env:CARGO_TARGET_DIR\release\bundle\nsis\*setup.exe" | Sort-Object LastWriteTime | Select-Object -Last 1
Copy-WithRetry $setup.FullName (Join-Path $win "kriptomerleg_${version}_windows_x64-setup.exe")
Copy-WithRetry "$env:CARGO_TARGET_DIR\release\kriptomerleg.exe" (Join-Path $win "kriptomerleg_${version}_windows_x64-portable.exe")

Write-Host "==> Linux build ($Distro WSL)" -ForegroundColor Cyan
$wslRoot = (wsl -d $Distro -- wslpath -a ($root -replace '\\', '/')).Trim()
wsl -d $Distro -- bash "$wslRoot/scripts/build-linux.sh" "$wslRoot"
if ($LASTEXITCODE -ne 0) { throw "Linux build sikertelen" }

Write-Host "==> Kész" -ForegroundColor Green
Get-ChildItem -Recurse (Join-Path $root "telepito") -File | Select-Object FullName, Length
