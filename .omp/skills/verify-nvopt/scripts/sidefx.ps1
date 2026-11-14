#Requires -Version 7
# Runs ON hal9k-win. Read-only listing (path|bytes|mtime) of every place nvopt can write, so a drive
# can prove it changed nothing outside its own first-run %APPDATA%\nvopt\config.toml.
$ErrorActionPreference = 'Stop'
$places = @(
    (Join-Path $env:APPDATA 'nvopt'),                                                       # config, watchlist, cache, exports
    (Join-Path $env:LOCALAPPDATA 'Lossless Scaling\Settings.xml'),                          # LS per-game profiles
    (Join-Path $env:LOCALAPPDATA 'NVIDIA Corporation\NVIDIA App\NvBackend\Recommendations'), # OPS metadata (I key, presets)
    'C:\Program Files (x86)\RivaTuner Statistics Server\Profiles',                          # RTSS caps (t/T keys)
    'C:\Program Files\RivaTuner Statistics Server\Profiles',
    'C:\Program Files (x86)\MSI Afterburner\RTSS\Profiles'
)
foreach ($p in $places) {
    if (-not (Test-Path $p)) { "$p|absent"; continue }
    Get-ChildItem -LiteralPath $p -Recurse -File -Force -ErrorAction SilentlyContinue |
        Sort-Object FullName |
        ForEach-Object { '{0}|{1}|{2:o}' -f $_.FullName, $_.Length, $_.LastWriteTimeUtc }
}
