#!/bin/sh
# Mac side: stop a drive.py ssh session that is still alive (by its recorded PID only), then run the
# Windows-side cleanup. Evidence under $XDG_STATE_HOME/verify-nvopt/evidence/ is kept.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
state=${XDG_STATE_HOME:-$HOME/.local/state}/verify-nvopt
if [ -f "$state/drive.pid" ]; then
  pid=$(cat "$state/drive.pid")
  if ps -p "$pid" -o command= 2>/dev/null | grep -q '^ssh -tt .*hal9k-win'; then
    kill "$pid" && echo "killed our ssh pid $pid"
  fi
  rm -f "$state/drive.pid"
fi
"$here/remote.sh" cleanup.ps1
echo "evidence kept: $state/evidence"
