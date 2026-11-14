# Launch and hardware summary

On start nvopt detects the GPU, CPU, RAM and display, scans Steam/Epic/GOG libraries and the NVIDIA App OPS data, prints a summary to the terminal, then opens the TUI with the hardware line in the status bar.

## Sub-features

- `launch-scan` pre-TUI lines: `Detecting hardware...`, `  GPU: …`, `  CPU: …`, `  Display: WxH@HzHz`, `  Tier: …`, `Scanning game libraries...`, `  Found N games (Steam: a, Epic: b, GOG: c)`, `Scanning NVIDIA OPS profiles...`, `  Found N game profiles`.
- `launch-tui` title ` nvopt │ GPU Game Settings Optimizer` and `─ Installed (N) ─` with `▸` on the first game.
- `launch-status` status line `<GPU> (<VRAM>MB) │ <CPU> │ <RAM>MB RAM │ WxH@HzHz │ Tier: <tier>`.
- `launch-hw-fail` when detection fails the status reads `Hardware detection failed` and the TUI still opens.
- `launch-quit` `q` or `Esc` leaves the alternate screen and exits 0.

## How to get to it (user POV)

- Run `nvopt.exe` in a terminal.

## Driving it with drive.py

Preconditions:

- `doctor.ps1` ends with `DOCTOR OK` and `NVIDIA_SMI=` shows the GPU.

- **Launch.** Run `drive.py 'wait:─ Installed (' 'wait:Tier:' snap:launched`. `01-launched.txt` shows the title, `─ Installed (18) ─` (at last proof) and a bottom line like ` NVIDIA GeForce RTX 5080 (16303MB) │ AMD Ryzen 7 7800X3D 8-Core Processor │ 31910MB RAM │ …@60Hz │ Tier: Ultra`.
- **Scan output.** The pre-TUI lines are overwritten by the TUI on screen; prove them from `raw.bin`: `strings raw.bin | grep -E 'Found [0-9]+ (games|game profiles)'`.
- **Quit.** `drive.py` sends `q` after the steps; `steps.txt` ends with `nvopt session exit status 0` and doctor afterwards reports `OUR_RUNNING_INSTANCES=0`.
- **Proof.** `01-launched.txt`, the grep over `raw.bin`, and an empty `sidefx-diff.txt` (or only `config.toml` on the very first launch).

## Gotchas

- The first launch on a machine creates `%APPDATA%\nvopt\config.toml`; that is the only write a launch makes, and cleanup undoes it only when verify-nvopt created it.
- Display resolution comes from PowerShell; over ssh it has reported `1024x768@60Hz` (the ssh session's view, not the monitor). Do not treat that as a detection bug without checking from a desktop console.
- Hardware detection runs `nvidia-smi` and several PowerShell queries; expect 5-10 s before `─ Installed (` appears.
- The TUI needs a terminal; running nvopt.exe without `-tt` fails in `enable_raw_mode`.
