# nvopt verification map

Recipes for proving nvopt's user-facing behavior on hal9k-win through `drive.py`. Read this index, then the matching feature file.

## Baseline preconditions

- `scripts/remote.sh build.ps1` has built `D:\nvopt` into `%TEMP%\verify-nvopt` and `scripts/remote.sh doctor.ps1` ends with `DOCTOR OK`.
- Record the `COMMIT` and `SRC_DIRTY` lines from doctor with the evidence: `D:\nvopt` may carry uncommitted work the Mac checkout lacks.
- Baseline data is the user's real machine: the Steam library (18 installed games at last proof), the NVIDIA App OPS data (27 profiles), RTSS and Lossless Scaling installs. Nothing is seeded; assertions use names that exist there (`Cyberpunk 2077`, `3DMark`).
- Every drive starts a fresh `nvopt.exe` through `drive.py`. Never drive an nvopt window the user opened.

## Driving conventions

- Steps are literal: `wait:` on the strings named in each recipe, never fixed sleeps for state.
- Read-only keys only: `j`, `k`, `Up`, `Down`, `Tab`, `/`, `Esc`, `q`, plus typing and `Enter` inside the search box. `drive.py` refuses the rest in normal mode.
- Off limits for this skill (they write to the user's real settings or files): `Enter`/`r` fetch (PCGW cache, OPS load), `p` pin, `e` export, `o` browser, `t`/`T` RTSS caps, `I` OPS resolution inject, `1`-`4` presets, `+`/`-`/`=` OPS tiers. A journey that needs one is reported as not verified, with the key named.

## Proof and skip reporting

- Keep the `NN-<label>.txt` screens before and after each action, plus `raw.bin`, `doctor.txt` and the `sidefx-*.txt` files from the same evidence directory.
- A pass needs `DRIVE OK` and `sidefx changes: 0 (unexpected: 0)` (or only the first-run `config.toml`).
- Report an unreachable path with the step list you ran and the screen `drive.py` dumped on failure.

## Feature entry contract

Each feature file starts with an H1 and one paragraph, then exactly four H2s in order: `Sub-features`, `How to get to it (user POV)`, `Driving it with drive.py` (starting with `Preconditions:`), `Gotchas`.

## Features

- [Launch and hardware summary](./launch-and-hardware.md): startup scans and the hardware status line.
- [Installed library and detail panel](./library-browse.md): moving through installed games and reading the per-game panel.
- [Section switching](./sections.md): `Tab` across Installed, Watchlist and Search results.
- [Steam store search](./steam-search.md): `/` search against the Steam Store, results list and selection. Proven end to end.
