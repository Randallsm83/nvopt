#Requires -Version 7
# Runs ON hal9k-win. Copies the checkout's Cargo.toml/Cargo.lock/src into %TEMP%\verify-nvopt\src
# and builds there, so nothing is written to the checkout or to the shared CARGO_HOME.
$ErrorActionPreference = 'Stop'
$repo = 'D:\nvopt'
$root = Join-Path $env:TEMP 'verify-nvopt'
$src  = Join-Path $root 'src'
. (Join-Path $PSScriptRoot 'srchash.ps1')

New-Item -ItemType Directory -Force -Path $src | Out-Null
Remove-Item -Recurse -Force (Join-Path $src 'src') -ErrorAction SilentlyContinue
Copy-Item (Join-Path $repo 'Cargo.toml'), (Join-Path $repo 'Cargo.lock') $src -Force
Copy-Item (Join-Path $repo 'src') (Join-Path $src 'src') -Recurse -Force

# The rustup proxies in ~/.cargo/bin are symlinks that cannot execute over ssh; ask rustup for the real binary.
$cargo = (& rustup.exe which cargo).Trim()
$env:CARGO_HOME = Join-Path $root 'cargo-home'
$env:CARGO_TARGET_DIR = Join-Path $root 'target'
$env:PATH = (Split-Path $cargo) + ';' + $env:PATH
Push-Location $src
try {
    $log = & $cargo build --release --locked 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    $log | Where-Object { $_ -match '^(error|\s+Finished)' }
    if ($code -ne 0) { $log | Select-Object -Last 40; throw "cargo build failed with exit $code" }
} finally { Pop-Location }

$exe = Join-Path $env:CARGO_TARGET_DIR 'release\nvopt.exe'
$h = Get-NvoptSrcHash $src
$commit = (& git -C $repo rev-parse HEAD).Trim()
$dirty = (& git -C $repo status --porcelain -- Cargo.toml Cargo.lock src) -join ' '
Set-Content -NoNewline -Path (Join-Path $root 'built-from.sha256') -Value $h
"EXE=$exe"
"SRC_SHA256=$h"
"COMMIT=$commit"
"SRC_DIRTY=$(if ($dirty) { $dirty } else { 'clean' })"
