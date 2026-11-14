#!/bin/sh
# Usage: remote.sh <build.ps1|doctor.ps1|prelaunch.ps1|sidefx.ps1|cleanup.ps1>
# Copies this skill's PowerShell helpers to hal9k-win:%TEMP%\verify-nvopt\scripts and runs one of them there.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
case "${1:-}" in build.ps1|doctor.ps1|prelaunch.ps1|sidefx.ps1|cleanup.ps1) ;; *) echo "usage: $0 build.ps1|doctor.ps1|prelaunch.ps1|sidefx.ps1|cleanup.ps1" >&2; exit 2 ;; esac
ssh -o BatchMode=yes -o ConnectTimeout=15 hal9k-win "if not exist %TEMP%\\verify-nvopt\\scripts mkdir %TEMP%\\verify-nvopt\\scripts"
scp -q "$here"/*.ps1 hal9k-win:AppData/Local/Temp/verify-nvopt/scripts/
exec ssh -o BatchMode=yes hal9k-win "pwsh -NoProfile -NonInteractive -File %TEMP%\\verify-nvopt\\scripts\\$1"
