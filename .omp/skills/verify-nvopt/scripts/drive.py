#!/usr/bin/env -S uv run -q --with pyte python
"""Drive the nvopt TUI on hal9k-win through `ssh -tt` and capture screens as text.

Usage: drive.py STEP [STEP ...]
  wait:TEXT   block until the rendered screen contains TEXT (30 s, else fail)
  gone:TEXT   block until the screen no longer contains TEXT
  key:NAME    send one key: down up tab esc enter backspace, or a single character
  type:TEXT   send TEXT one char at a time (only valid in search mode)
  snap:LABEL  save the rendered screen to NN-LABEL.txt
  sleep:SECS  pause

Read-only guard: in normal mode only j k / Tab Esc q Up Down are accepted. Enter (writes the
PCGW cache), p e o r t T I 1-4 + - = (pin/export/browser/RTSS/OPS/presets) are refused.
After the steps it presses q, waits for exit, and diffs sidefx.ps1 listings taken before/after.
Evidence: $XDG_STATE_HOME/verify-nvopt/evidence/<UTC timestamp>/ (printed as EVIDENCE=...).
"""
import fcntl, json, os, pty, select, signal, struct, subprocess, sys, termios, time
from datetime import datetime, timezone

import pyte

HERE = os.path.dirname(os.path.abspath(__file__))
STATE = os.path.join(os.environ.get("XDG_STATE_HOME") or os.path.expanduser("~/.local/state"), "verify-nvopt")
COLS, ROWS = 160, 45
KEYS = {"down": b"\x1b[B", "up": b"\x1b[A", "tab": b"\t", "esc": b"\x1b", "enter": b"\r", "backspace": b"\x7f"}
NORMAL_OK = {"j", "k", "/", "tab", "esc", "q", "up", "down"}


def remote(script):
    r = subprocess.run([os.path.join(HERE, "remote.sh"), script], capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def main(steps):
    ev = os.path.join(STATE, "evidence", datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    os.makedirs(ev, exist_ok=True)
    print(f"EVIDENCE={ev}", flush=True)
    log = open(os.path.join(ev, "steps.txt"), "w")

    def note(msg):
        log.write(msg + "\n"); log.flush(); print(msg, flush=True)

    for name in ("doctor.ps1", "prelaunch.ps1"):
        code, out = remote(name)
        open(os.path.join(ev, name.replace(".ps1", ".txt")), "w").write(out)
        if code:
            note(f"FAIL {name} exit {code}:\n{out}"); return 1
    _, before = remote("sidefx.ps1")
    open(os.path.join(ev, "sidefx-before.txt"), "w").write(before)

    screen = pyte.Screen(COLS, ROWS)
    stream = pyte.ByteStream(screen)
    raw = open(os.path.join(ev, "raw.bin"), "wb")
    pid, fd = pty.fork()
    if pid == 0:
        os.execvp("ssh", ["ssh", "-tt", "-o", "BatchMode=yes", "hal9k-win",
                          r"%TEMP%\verify-nvopt\target\release\nvopt.exe"])
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
    open(os.path.join(STATE, "drive.pid"), "w").write(str(pid))

    def pump(secs):
        end = time.time() + secs
        while time.time() < end:
            r, _, _ = select.select([fd], [], [], 0.1)
            if not r:
                continue
            try:
                data = os.read(fd, 65536)
            except OSError:
                return False
            if not data:
                return False
            raw.write(data); stream.feed(data)
        return True

    def text():
        return "\n".join(line.rstrip() for line in screen.display)

    def until(pred, label, timeout=30):
        end = time.time() + timeout
        while time.time() < end:
            if pred(text()):
                return True
            if not pump(0.3):
                break
        note(f"FAIL {label}; screen:\n{text()}")
        return False

    snaps, mode, ok = 0, "normal", True
    for step in steps:
        kind, _, arg = step.partition(":")
        if kind == "wait":
            ok = until(lambda s: arg in s, f"wait:{arg}")
        elif kind == "gone":
            ok = until(lambda s: arg not in s, f"gone:{arg}")
        elif kind == "key":
            if mode == "normal" and arg not in NORMAL_OK:
                note(f"REFUSED key:{arg} (mutating or not read-only in normal mode)"); ok = False
            else:
                os.write(fd, KEYS.get(arg, arg.encode()))
                if mode == "normal" and arg == "/":
                    mode = "search"
                elif mode == "search" and arg in ("enter", "esc"):
                    mode = "normal"
                pump(0.5)
        elif kind == "type":
            if mode != "search":
                note(f"REFUSED type:{arg} outside search mode"); ok = False
            else:
                for ch in arg:
                    os.write(fd, ch.encode()); pump(0.05)
                pump(0.5)
        elif kind == "snap":
            snaps += 1
            path = os.path.join(ev, f"{snaps:02d}-{arg}.txt")
            open(path, "w").write(text() + "\n")
            note(f"snap {path}")
        elif kind == "sleep":
            pump(float(arg))
        else:
            note(f"unknown step {step}"); ok = False
        note(f"{'ok' if ok else 'FAILED'} {step}")
        if not ok:
            break

    if mode == "search":
        os.write(fd, KEYS["esc"]); pump(0.5)
    os.write(fd, b"q")
    done, status = 0, 0
    for _ in range(30):  # ConPTY-backed ssh sessions can take a few seconds to close after the app exits
        pump(0.5)
        done, status = os.waitpid(pid, os.WNOHANG)
        if done:
            break
    if not done:
        os.kill(pid, signal.SIGTERM); _, status = os.waitpid(pid, 0)
        note("ssh did not exit after q; terminated our ssh pid")
    os.unlink(os.path.join(STATE, "drive.pid"))
    raw.close()
    note(f"nvopt session exit status {os.waitstatus_to_exitcode(status)}")

    _, after = remote("sidefx.ps1")
    open(os.path.join(ev, "sidefx-after.txt"), "w").write(after)
    b, a = set(before.splitlines()), set(after.splitlines())
    changed = sorted((a - b) | (b - a))
    open(os.path.join(ev, "sidefx-diff.txt"), "w").write("\n".join(changed) + "\n")
    # Only the first-run default config is expected: the dir's "absent" line and its config.toml.
    expected = (r"AppData\Roaming\nvopt", r"AppData\Roaming\nvopt\config.toml")
    unexpected = [c for c in changed if not c.split("|")[0].endswith(expected)]
    note(f"sidefx changes: {len(changed)} (unexpected: {len(unexpected)})")
    for c in unexpected:
        note(f"UNEXPECTED WRITE {c}")
    json.dump({"steps": steps, "ok": ok and not unexpected, "snaps": snaps}, open(os.path.join(ev, "run.json"), "w"), indent=1)
    note("DRIVE OK" if ok and not unexpected else "DRIVE FAILED")
    return 0 if ok and not unexpected else 1


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__); sys.exit(2)
    sys.exit(main(sys.argv[1:]))
