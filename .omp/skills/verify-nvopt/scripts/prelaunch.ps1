#Requires -Version 7
# Runs ON hal9k-win before every launch. nvopt creates %APPDATA%\nvopt\config.toml on first run and
# dirs::config_dir cannot be redirected, so record whether that dir existed BEFORE our first launch.
# The first record wins; cleanup.ps1 removes %APPDATA%\nvopt only when this says ABSENT.
$ErrorActionPreference = 'Stop'
$root   = Join-Path $env:TEMP 'verify-nvopt'
$marker = Join-Path $root 'appdata-preexisting.txt'
$cfg    = Join-Path $env:APPDATA 'nvopt'
New-Item -ItemType Directory -Force -Path $root | Out-Null
if (-not (Test-Path $marker)) {
    Set-Content -NoNewline -Path $marker -Value $(if (Test-Path $cfg) { 'PRESENT' } else { 'ABSENT' })
}
$exe = Join-Path $root 'target\release\nvopt.exe'
if (-not (Test-Path $exe)) { throw "not built: run remote.sh build.ps1 first" }
"APPDATA_NVOPT_BEFORE_FIRST_RUN=$(Get-Content -Raw $marker)"
"EXE=$exe"
