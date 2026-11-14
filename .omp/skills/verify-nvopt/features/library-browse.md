# Installed library and detail panel

The left pane lists installed games sorted by name; the user moves the `▸` selection with `j`/`k` or the arrow keys and the right pane shows that game's name, tool status (RTSS cap, Lossless Scaling profile) and, until data is fetched, a prompt to press Enter.

## Sub-features

- `browse-list` `─ Installed (N) ─` followed by one row per game with a data icon: `○` none, `●` PCGW data, `◆` NVIDIA OPS.
- `browse-move` `j`/`Down` and `k`/`Up` move `▸`; it stops at the first and last rows.
- `browse-detail` the right pane heading is the selected game's name.
- `browse-tools` `─ Tools ─` with `● RTSS  no cap set …` or `◆ RTSS  N FPS cap …`, and `● LS` profile status (only when RTSS is installed or Lossless Scaling has settings).
- `browse-prompt` `Press Enter to fetch game info & recommendations` and the `─ Keys ─` help.

## How to get to it (user POV)

- Launch nvopt; the Installed section is active.

## Driving it with drive.py

Preconditions:

- `doctor.ps1` ends with `DOCTOR OK`; the Steam library on hal9k-win contains `3DMark` and `Cyberpunk 2077`.

- **First row.** Run `drive.py 'wait:─ Installed (' snap:first`. `▸` is on `3DMark` and the detail heading is `3DMark`.
- **Move down.** Add `key:j key:j key:j key:j key:j key:j 'wait:▸' snap:down6`. `▸` is on `Cyberpunk 2077` (seventh row at last proof) and the heading reads `Cyberpunk 2077`.
- **Move up.** Add `key:k snap:up1`. `▸` is on the row above.
- **Top clamp.** Add `key:up key:up key:up key:up key:up key:up key:up key:up snap:top`. `▸` is on `3DMark` again.
- **Tools.** In `snap:first`, the panel shows `─ Tools ─`, `● RTSS  no cap set  [t = set refresh-3 | T = clear]`, `● LS    no profile for this game`.
- **Proof.** The snaps in order plus an empty `sidefx-diff.txt`.

## Gotchas

- Row order is case-insensitive by name and includes unnamed apps as `appid_<id>`; count rows from the screen rather than hard-coding offsets when the library changes.
- OPS (`◆`) and PCGW (`●`) icons and the settings panels appear only after `Enter` (fetch), which this skill does not press. A fresh launch shows `○` for every game.
- `t`/`T` next to the RTSS line write RTSS profile files; never press them here.
