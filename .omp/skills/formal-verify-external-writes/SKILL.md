---
name: formal-verify-external-writes
description: "Use when changing nvopt code that writes into another tool's files: RTSS profiles (rtss.rs set_fps_limit, parse_profile, save_profile), NVIDIA App OPS metadata (nvops.rs inject_resolution, interpolate_pops_1based, pick_pops_for_resolution), or when adding a caller of LosslessScaling::save_profile. Also when an injected resolution, an OPS tier pick, or an RTSS cap looks wrong."
---

# Verify nvopt's writes into other tools' files

nvopt rewrites files it does not own: the NVIDIA App's OPS metadata (`I` key) and RTSS application profiles (`t` / `T` keys). There is no backup and no undo, so a wrong write costs the user another tool's settings. This skill checks nine properties of that code by running the real functions on generated inputs.

## The covered code is uncommitted

`src/nvops.rs` and `src/rtss.rs` do not exist at nvopt HEAD (`06726e7`). They live only in the uncommitted working tree at `hal9k-win` `D:\nvopt`, read on 2026-11-13. There is no sync commit to record, so the sync point is the sha256 of each covered file (carriage returns removed) in `covered-sources.sha256`.

## Does your change touch this?

Run the check when a change touches any of:

| File | Functions |
|---|---|
| `src/nvops.rs` | `NvOpsIndex::inject_resolution`, `inject_resolution_for_game`, `patch_ops_in_file`, `patch_ops_in_file_nested`, `inject_into_ops_array`, `interpolate_pops_1based`, `pick_pops_for_resolution`, `parse_resolution`, `OpsEntry` |
| `src/rtss.rs` | `RtssManager::set_fps_limit`, `remove_fps_limit`, `parse_profile`, `save_profile`, `RtssProfile` |
| `src/app.rs` | `game_exe`, `apply_rtss_for_selected`, `clear_rtss_for_selected`, `inject_resolution_to_nvidia`, `resolved_display_res` (the callers; the properties assume the inputs these pass) |
| `Cargo.toml` / `Cargo.lock` | the `serde_json` version or its features |

`check.sh` prints `DRIFT` when `nvops.rs` or `rtss.rs` differs from the coverage record. `app.rs` and `models.rs` are not hashed.

## Run it

From the repo root. A run takes about 10 seconds; the first one also downloads and builds the pinned crates:

```sh
source ~/.config/zsh/agent-env.zsh && .omp/skills/formal-verify-external-writes/scripts/check.sh
```

Sources come from this checkout's `src/` when it holds the covered files, otherwise from a read-only snapshot of the `hal9k-win` working tree (ssh + tar; nothing is written there). To check a change before it lands, copy `nvops.rs`, `rtss.rs` and `models.rs` into a scratch directory under `~/.local/state/formal-nvopt/`, edit there, and pass `--src <that directory>`.

`--replay <dir>` also runs the injection on a temporary copy of a real `%LOCALAPPDATA%\NVIDIA Corporation\NVIDIA App\NvBackend\Recommendations` folder. Give it a copy pulled off the Windows machine; the directory is only read.

| Exit | Meaning |
|---|---|
| 0 | Holding properties pass and the known findings still fail. |
| 1 | A holding property failed (`BROKEN`), a property did not run, or the build failed. |
| 2 | A known finding passes now (`FIXED?`). |

Each run writes `summary.txt` and `cargo-test.log` under `~/.local/state/verify-nvopt/formal-verify-external-writes/runs/<UTC time>/`. The harness is assembled and built under `~/.local/state/formal-nvopt/external-writes/`, never in the checkout. No real RTSS folder, registry key, or NVIDIA folder is opened: each test builds its files in a temporary directory.

## What is checked

Tool: `proptest` 1.12.0 running the real `nvops.rs` and `rtss.rs` (copied verbatim, with the property files attached as child modules so they reach private functions). Each key press runs to completion on one thread, so there is no ordering question for a model checker; the questions are about what the code does for every input. No Rust verifier (Kani, Creusot, Prusti) is on this machine, and this code is strings, JSON and files, so the real code is tested with generated inputs. Bounds: 512 cases per property, seed 20261113 (`PROPTEST_CASES` and `PROPTEST_RNG_SEED` override them). A pass covers those inputs, not all inputs.

| Property (test name) | In plain words | Harm if broken |
|---|---|---|
| `inject_changes_only_the_new_entry` | Pressing `I` puts one entry for the display resolution in front of each ops array that lacks it and changes nothing else in the NVIDIA App's files. | Every game's NVIDIA recommendations are corrupted at once, with nothing to restore from. |
| `inject_twice_is_inject_once` | Pressing `I` again changes no byte. | Duplicate entries pile up on every press. |
| `picked_tier_is_valid_and_listed_resolution_wins` | The tier nvopt picks exists, and a resolution NVIDIA lists gets NVIDIA's tier. | nvopt shows an empty or wrong settings list for the user's own display. |
| `set_fps_limit_stays_inside_profiles_dir` | Whatever the executable name, a cap writes one `.cfg` directly inside RTSS's `Profiles` folder. | A crafted game name overwrites a file elsewhere on disk. |
| `cap_roundtrip_on_nvopt_written_profile` | On a profile nvopt wrote, `t` changes only the cap, a second `t` changes no byte, and `T` leaves it unlimited. | Scanline sync or OSD flags vanish, or a cap stays after it was cleared. |

Generated shapes follow 27 real game folders read on hal9k-win on 2026-11-13: `regular` or `regular_rtx` plus `etag`; blocks with `version`, `pops`, `ops`; ops entries with `resolution`, `pops` (1 to 56), `rate`, `belowMinSpec`; arrays unsorted with up to 11 entries; the nested `<variant>/metadata.json` wrapper; every number an integer. Unknown fields, junk entries and non-array `ops` are generated on top. Executable names for the round trip have the shape `App::game_exe` produces.

## Findings in the current code

These four properties fail today. `check.sh` prints them as `FINDING` and still exits 0.

1. **`injected_pops_within_bracket`, `picked_tier_within_bracket`.** The tier for an unlisted resolution should lie between the tiers of the nearest listed resolutions by pixel count. It does not when the display has the same pixel count as a listed resolution but different text, such as a rotated monitor.
   - ops lists `1024x768` with pops 1 and `1280x1024` with pops 2; the display is `1024x1280`.
   - No text matches, the nearest entry below and above are both `1280x1024`, and the `_ =>` arm of `pick_pops_for_resolution` / `interpolate_pops_1based` returns `parsed.first()`, the smallest resolution.
   - nvopt picks, and `I` writes, the pops of `1024x768` instead of `1280x1024`.
   - On a copy of the 27 real folders, injecting `2160x3840` wrote a pops outside the 4K bracket in 10 games (52 where `3840x2160` has 25, in the worst one).
   - Replacing `parsed.first()` with `lower.or(upper)` in both arms makes both properties pass (tried on a scratch copy).
   - Harm: on a rotated display nvopt recommends, and writes into the NVIDIA App's data for every game, a far heavier quality tier than NVIDIA's own.
2. **`inject_keeps_fractional_numbers`.** `I` re-prints the whole file through `serde_json`, which does not read every fractional number exactly: `-1.1338274384357326e+72` elsewhere in the file became `-1.1338274384357328e+72`. Harm: unrelated numbers in the NVIDIA App's files change silently. No real file holds a fractional number today (487 numbers, all integers).
3. **`set_fps_limit_keeps_foreign_keys`.** `t` on a profile that already exists rewrites it from the few fields `RtssProfile` models.
   - `Profiles\<name>.cfg` holds the line `A=`.
   - `set_fps_limit` writes `[<name>]`, `AppName=`, `FramerateLimit=…`, `EnableOSD=0`, `ShowFPS=0`, `ShowFrametime=0`.
   - `A=` is gone, and `AppName` is blank because the old file had none.
   - Harm: every setting RTSS or the user kept in that profile is lost. It only bites when a file already exists under the name nvopt derives from the game's title; whether RTSS's own profiles ever carry such a name was not checked (hal9k-win has no RTSS at the three install paths nvopt probes).

## When a property fails

1. Read the `Test failed:` text in the summary; it states the input and what the code did in the code's words. The full shrunk input is in `cargo-test.log`.
2. Work the input through the function by hand.
3. It is a real bug when the code now does what the plain-words sentence forbids. Fix the code.
4. It is a modelling error when the test's picture of the world is out of date. The usual causes:
   - `usable_entry` in `harness/props/nvops_props.rs` no longer matches the fields of `OpsEntry`.
   - `MODELLED_KEYS` in `harness/props/rtss_props.rs` no longer matches the keys `parse_profile` reads.
   - `game_exe()` in `rtss_props.rs` no longer matches `App::game_exe`.
   - NVIDIA changed the file layout: confirm with `--replay` on a fresh copy of a real store, then update the generators.
5. `FIXED?` on a finding: confirm the fix is the cause, change its row in `properties.tsv` from `finding` to `holds`, and move it out of the findings above.

## When the change is intended

1. Edit the property in `harness/props/`, or add one and list it in `properties.tsv`.
2. Show the new or changed property failing on a broken variant: copy the sources to a scratch directory, break the guarded code there, run `check.sh --src <scratch>`, and see `BROKEN` on that property.
3. Re-read the tables above against the new code and update them.
4. Only then run `check.sh --record`. It rewrites `covered-sources.sha256` and refuses unless the run is as expected. Once the code is committed, add the commit to the section on uncommitted code.

`/maintain-formal-verification-skill` brings this skill back in line after changes that did not recheck it.

## Not covered

- `LosslessScaling::save_profile` (`src/lossless.rs`). Nothing calls it today. It re-serialises `Settings.xml` from the ten fields `LsProfile` models; a real profile on hal9k-win has 41 elements and the root has many global settings, so wiring it to a key would erase the rest. Add a property before adding a caller.
- A crash or full disk during `std::fs::write`, which truncates before it writes. Neither store is written atomically or backed up.
- Top-level and nested `metadata.json` disagreeing after an error: `inject_resolution_for_game` discards the nested file's error.
- Whether RTSS or the NVIDIA App accept what nvopt writes: the `.cfg` name and key names, the JSON key order and spacing after a rewrite (documents are compared as JSON values, not bytes), the unchanged `etag`.
- Numbers wider than 64 bits.
- Windows path rules. The check was run on macOS only; reserved names, case folding and `\` as a separator are not exercised. Registry lookups are replaced by `harness/winreg-stub`, so `RtssManager::new` and install detection are not checked.
- `src/export.rs`, `src/drs.rs`, presets, the UI.
