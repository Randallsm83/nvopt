---
name: verify-nvopt
description: Drive the nvopt TUI (NVIDIA per-game settings optimizer, Rust/ratatui) on hal9k-win from this Mac over `ssh -tt`, read-only screens only, and capture text screenshots plus a side-effect diff as evidence. Use when you need to prove nvopt's launch, library list, section switching or Steam search behaves as a user sees it, or when anything about the hal9k-win build looks off.
---

# verify-nvopt

nvopt is a Windows-only terminal UI. `winreg` is an unconditional dependency, so `cargo build` on this Mac fails (`could not compile winreg`); it also shells out to `nvidia-smi`, PowerShell and the registry. Every run therefore happens on **hal9k-win**, driven from the Mac through a PTY around `ssh -tt hal9k-win nvopt.exe` (Windows OpenSSH gives it a ConPTY, which crossterm renders into normally; no desktop session or scheduled task is needed).

**Which code you are proving.** The build comes from the Windows checkout `D:\nvopt`, not the Mac checkout. Proven on 2026-11-13 at commit `06726e7f9ebb7f23df56157ad2889a3e4b5a2a95` with a heavily **dirty** working tree (modified `Cargo.toml`, `src/*.rs`; untracked `src/lossless.rs`, `src/nvops.rs`, `src/rtss.rs`; source hash `030DD5155FFC4E9E358A553CD91D361B6AAFDA292F1459ADC2FBA09C9940BB82`). The Mac checkout at the same commit lacks RTSS, Lossless Scaling, NVIDIA OPS, presets and Epic/GOG scanning. `doctor.ps1` prints the commit, dirty files and source hash of what you build; record them with your evidence.

**Read-only rule.** nvopt has no dry-run mode, and these keys write outside the app or to the user's real settings: `Enter` and `r` (PCGW cache in `%APPDATA%\nvopt\cache`, `r` also deletes it), `p` (watchlist.toml), `e` (.nip export), `o` (opens a browser), `t`/`T` (RTSS profile files under Program Files), `I` (patches NVIDIA App OPS metadata), `1`-`4` and `+`/`-`/`=` (OPS presets/tiers). `drive.py` refuses all of them in normal mode. Do not bypass it; if a task needs one of those paths, stop and ask.

The one unavoidable write: the first launch creates `%APPDATA%\nvopt\config.toml` (default config). `dirs::config_dir()` uses `SHGetKnownFolderPath`, so `APPDATA`/`USERPROFILE` overrides do not redirect it (probed). `prelaunch.ps1` records whether `%APPDATA%\nvopt` existed before the first verify launch; cleanup removes it only if it did not.

All helpers live in `scripts/` next to this file. Run them from the Mac:

```sh
S=<project>/.omp/skills/verify-nvopt/scripts   # e.g. /Users/ranmil/projects/nvopt/.omp/skills/verify-nvopt/scripts
```

## Launch

1. Build in `%TEMP%` on hal9k-win (copies `D:\nvopt\{Cargo.toml,Cargo.lock,src}` to `%TEMP%\verify-nvopt\src`, private `CARGO_HOME`, `--locked`; nothing is written to `D:\nvopt`):
   ```sh
   $S/remote.sh build.ps1
   ```
   Ready when it prints `Finished`, `EXE=C:\Users\ranmil\AppData\Local\Temp\verify-nvopt\target\release\nvopt.exe`, `COMMIT=`, `SRC_DIRTY=`. A cold build takes about 30 s.
2. There is no server. Each drive starts its own `nvopt.exe` through `drive.py`, which runs `prelaunch.ps1` first (records the `%APPDATA%\nvopt` marker, checks the exe exists). The TUI is ready when the screen shows `─ Installed (` and the bottom status line shows the hardware summary (`… │ Tier: …`) or `Hardware detection failed`.
3. Teardown of one session: `drive.py` presses `q` and waits for ssh to close; if it does not within 15 s it kills only its own ssh PID.

Ports: none. Instances are isolated per ssh session, but they share `%APPDATA%\nvopt`, so read-only drives may run side by side; never run a mutating key while another session is open.

## Doctor

```sh
$S/remote.sh doctor.ps1
```

Read-only. Prints `COMMIT`, `SRC_DIRTY`, `CHECKOUT_SRC_SHA256` vs `BUILT_SRC_SHA256`, `NVIDIA_SMI`, `OUR_RUNNING_INSTANCES` (nvopt.exe processes whose path is our `%TEMP%` exe), and the `%APPDATA%\nvopt` state. Ends with `DOCTOR OK`, or `DOCTOR FAIL: …` and exit 1 when the exe is missing or older than `D:\nvopt`'s sources (rebuild). If ssh itself fails, hal9k-win is off or off the tailnet; nothing here can fix that.

## Drive

`drive.py` (runs under `uv run --with pyte`, via its shebang) spawns `ssh -tt hal9k-win %TEMP%\verify-nvopt\target\release\nvopt.exe` in a 160×45 PTY, feeds output into a `pyte` screen, and executes steps in order:

| step | meaning |
|---|---|
| `wait:TEXT` / `gone:TEXT` | block (30 s) until the rendered screen does / does not contain TEXT; fail with the screen dumped |
| `key:NAME` | `down up tab esc enter backspace` or one character; normal mode accepts only `j k / tab esc q up down` |
| `type:TEXT` | type into the search box (only after `key:/`) |
| `snap:LABEL` | write the rendered screen to `NN-LABEL.txt` |
| `sleep:SECS` | pause |

Stable handles from `src/ui.rs` / `src/app.rs` (Windows tree): title ` nvopt │ GPU Game Settings Optimizer`; list headers `─ Installed (N) ─`, `─ Watchlist (N) ─` (only when non-empty), `─ Search (N) ─` (only after a search); selection marker `▸ `; per-game data icon `○` none, `●` PCGW, `◆` NVIDIA OPS; search box ` Search: `; status `Found N results for 'Q'` / `Search error: …`; detail panel `─ Tools ─`, `● RTSS`, `● LS`, `Press Enter to fetch game info & recommendations`, `─ Keys ─`.

Example (the proven journey, see `features/steam-search.md`):

```sh
$S/drive.py 'wait:─ Installed (' snap:launched key:/ 'wait: Search: ' type:Cyberpunk snap:query-typed \
  key:enter 'wait:─ Search (' 'wait:results for' snap:results key:down snap:second-result key:tab 'wait:Installed' snap:after-tab
```

It prints `EVIDENCE=<dir>`, one `ok`/`FAILED` line per step, `sidefx changes: N (unexpected: M)` and finally `DRIVE OK` (exit 0) or `DRIVE FAILED` (exit 1).

## Evidence

Each drive writes `${XDG_STATE_HOME:-$HOME/.local/state}/verify-nvopt/evidence/<UTC timestamp>/`:

- `doctor.txt`, `prelaunch.txt`: commit, dirty files, source hash and appdata marker for this run.
- `NN-<label>.txt`: rendered screens in step order (the action and the resulting state, not just the last screen).
- `raw.bin`: the full byte stream from the PTY, including the pre-TUI stderr lines (`Detecting hardware...`, `Found N games (Steam: …)`, `Found N game profiles`).
- `steps.txt`, `run.json`: what was run and the verdict.
- `sidefx-before.txt`, `sidefx-after.txt`, `sidefx-diff.txt`: path|bytes|mtime of every place nvopt can write (`%APPDATA%\nvopt`, Lossless Scaling `Settings.xml`, NVIDIA App `NvBackend\Recommendations`, RTSS `Profiles`). Any change other than the first-run `config.toml` fails the drive with `UNEXPECTED WRITE`.

Proof standard: drive the real binary from the real checkout through the keyboard; a pass needs the screens that show the action and its result plus a clean `sidefx-diff.txt`. Search hits the live Steam Store API (`store.steampowered.com/api/storesearch`, read-only GET); no mocks. pyte renders the nerd-font game icon one cell wide, so columns after it drift by one; compare text, not columns.

## Cleanup

```sh
$S/cleanup.sh
```

Kills a still-running drive's ssh by the PID in `…/verify-nvopt/drive.pid` (only if it is our `ssh -tt … hal9k-win`), then `cleanup.ps1` on hal9k-win: stops nvopt.exe processes whose path is our `%TEMP%` exe (by PID), deletes any `verify-nvopt-*` scheduled task, removes `%APPDATA%\nvopt` only if the marker says it was absent before, and deletes `%TEMP%\verify-nvopt` (build, cargo-home, marker). It ends with `CLEANUP OK` and `evidence kept: …`; the Mac evidence directory is never touched. The next run needs `build.ps1` again.

## Helpers

| script | where it runs | invocation |
|---|---|---|
| `remote.sh` | Mac | `$S/remote.sh build.ps1\|doctor.ps1\|prelaunch.ps1\|sidefx.ps1\|cleanup.ps1`: copies all `*.ps1` to `%TEMP%\verify-nvopt\scripts` and runs one with `pwsh -NoProfile -NonInteractive` |
| `build.ps1` | hal9k-win | via `remote.sh build.ps1` (uses `rustup which cargo`: the `~/.cargo/bin` proxies are symlinks that fail over ssh with "cannot execute the specified program") |
| `doctor.ps1` | hal9k-win | via `remote.sh doctor.ps1` |
| `prelaunch.ps1` | hal9k-win | called by `drive.py`; first record of the `%APPDATA%\nvopt` marker wins |
| `sidefx.ps1` | hal9k-win | called by `drive.py` before and after; `remote.sh sidefx.ps1` for a manual listing |
| `srchash.ps1` | hal9k-win | dot-sourced by build/doctor: SHA256 over Cargo.toml, Cargo.lock, src/*.rs |
| `drive.py` | Mac | `$S/drive.py STEP…` (needs `uv`; pyte is fetched ephemerally) |
| `cleanup.sh` / `cleanup.ps1` | Mac / hal9k-win | `$S/cleanup.sh` |

Feature map: `features/README.md`. Keep it honest as the app changes with `/maintain-verification-skill`.
