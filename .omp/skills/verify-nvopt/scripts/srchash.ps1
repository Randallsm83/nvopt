# Dot-sourced by build.ps1 and doctor.ps1: one SHA256 over Cargo.toml, Cargo.lock and src/*.rs (sorted).
function Get-NvoptSrcHash([string]$dir) {
    $files = @(Join-Path $dir 'Cargo.toml'; Join-Path $dir 'Cargo.lock') +
             @(Get-ChildItem (Join-Path $dir 'src') -Filter *.rs -File | Sort-Object Name | ForEach-Object FullName)
    $joined = ($files | ForEach-Object { (Get-FileHash -Algorithm SHA256 $_).Hash }) -join "`n"
    $stream = [IO.MemoryStream]::new([Text.Encoding]::UTF8.GetBytes($joined))
    (Get-FileHash -Algorithm SHA256 -InputStream $stream).Hash
}
