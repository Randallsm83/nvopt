#Requires -Version 7
# Runs ON hal9k-win. Removes only what verify-nvopt created: stray instances of OUR exe (by PID),
# any verify-nvopt-* scheduled task, %APPDATA%\nvopt if it did not exist before our first launch,
# and the %TEMP%\verify-nvopt scratch tree. Evidence lives on the Mac and is not touched.
$ErrorActionPreference = 'Stop'
$root   = Join-Path $env:TEMP 'verify-nvopt'
$exe    = Join-Path $root 'target\release\nvopt.exe'
$marker = Join-Path $root 'appdata-preexisting.txt'
$cfg    = Join-Path $env:APPDATA 'nvopt'

Get-CimInstance Win32_Process -Filter "Name='nvopt.exe'" | Where-Object ExecutablePath -eq $exe |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force; "killed our nvopt.exe pid $($_.ProcessId)" }

Get-ScheduledTask -TaskName 'verify-nvopt-*' -ErrorAction SilentlyContinue |
    ForEach-Object { Unregister-ScheduledTask -TaskName $_.TaskName -Confirm:$false; "deleted task $($_.TaskName)" }

if ((Test-Path $marker) -and (Get-Content -Raw $marker) -eq 'ABSENT') {
    if (Test-Path $cfg) { Remove-Item -Recurse -Force $cfg; "removed $cfg (created by verify-nvopt)" }
} elseif (Test-Path $marker) {
    "left $cfg untouched (existed before verify-nvopt)"
} else {
    "no marker: never launched by verify-nvopt, left $cfg untouched"
}

Set-Location $env:TEMP
if (Test-Path $root) { Remove-Item -Recurse -Force $root; "removed $root" }
"APPDATA_NVOPT_EXISTS_NOW=$(Test-Path $cfg)"
'CLEANUP OK'
