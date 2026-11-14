#Requires -Version 7
# Runs ON hal9k-win. Read-only: is the %TEMP% build worth driving? Exit 1 when it is not.
$ErrorActionPreference = 'Stop'
$repo = 'D:\nvopt'
$root = Join-Path $env:TEMP 'verify-nvopt'
$exe  = Join-Path $root 'target\release\nvopt.exe'
. (Join-Path $PSScriptRoot 'srchash.ps1')
$bad = @()

"COMMIT=$((& git -C $repo rev-parse HEAD).Trim())"
$dirty = (& git -C $repo status --porcelain -- Cargo.toml Cargo.lock src) -join ' '
"SRC_DIRTY=$(if ($dirty) { $dirty } else { 'clean' })"
$now = Get-NvoptSrcHash $repo
"CHECKOUT_SRC_SHA256=$now"

if (Test-Path $exe) {
    $built = Get-Content -Raw (Join-Path $root 'built-from.sha256') -ErrorAction SilentlyContinue
    "BUILT_SRC_SHA256=$built"
    if ($built -ne $now) { $bad += 'build is stale vs D:\nvopt: run remote.sh build.ps1' }
} else { $bad += 'not built: run remote.sh build.ps1' }

$gpu = & nvidia-smi --query-gpu=name,driver_version --format=csv,noheader 2>$null
"NVIDIA_SMI=$(if ($LASTEXITCODE -eq 0) { $gpu } else { 'FAILED (hardware detection will fail; TUI still launches)' })"

$ours = @(Get-CimInstance Win32_Process -Filter "Name='nvopt.exe'" | Where-Object ExecutablePath -eq $exe)
"OUR_RUNNING_INSTANCES=$($ours.Count) $(($ours | ForEach-Object ProcessId) -join ',')"
$marker = Join-Path $root 'appdata-preexisting.txt'
"APPDATA_NVOPT_EXISTS_NOW=$(Test-Path (Join-Path $env:APPDATA 'nvopt'))"
"APPDATA_NVOPT_BEFORE_FIRST_RUN=$(if (Test-Path $marker) { Get-Content -Raw $marker } else { 'not recorded yet' })"

if ($bad) { $bad | ForEach-Object { "DOCTOR FAIL: $_" }; exit 1 }
'DOCTOR OK'
