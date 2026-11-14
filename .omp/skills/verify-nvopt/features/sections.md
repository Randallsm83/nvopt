# Section switching

`Tab` cycles the active list between Installed, Watchlist and Search results. The selection resets to the first row of the new section, and the detail pane follows it.

## Sub-features

- `tab-cycle` order is Installed → Watchlist → Search results → Installed.
- `tab-empty` a section with no rows is hidden from the list; tabbing into it leaves no `▸` and the detail pane reads `Select a game and press Enter to fetch recommendations`.
- `tab-search` after a search, `Tab` from Installed (with an empty watchlist) needs two presses to reach `─ Search (N) ─`.
- `tab-reset` each switch puts `▸` on the first row of the new section.

## How to get to it (user POV)

- Press `Tab` in normal mode.

## Driving it with drive.py

Preconditions:

- `doctor.ps1` ends with `DOCTOR OK`; `%APPDATA%\nvopt\watchlist.toml` is absent (doctor's appdata lines; the default state), so the Watchlist is empty.

- **Into empty Watchlist.** Run `drive.py 'wait:─ Installed (' snap:installed key:tab 'wait:Select a game and press Enter' snap:watchlist`. No `▸` on screen and no `─ Watchlist (` header.
- **Into empty Search.** Add `key:tab snap:search-empty`. Same empty detail text.
- **Back to Installed.** Add `key:tab 'gone:Select a game and press Enter' snap:installed-again`. `▸` is on the first installed game.
- **With results.** Run a search first (see [steam-search.md](./steam-search.md)), then `key:tab key:tab key:tab 'wait:▸ Cyberpunk 2077' snap:back-in-search`: Installed → Watchlist → Search lands `▸` on the first result.
- **Proof.** The snaps plus an empty `sidefx-diff.txt`.

## Gotchas

- Section headers differ only by bold styling, which the text snaps do not keep; prove the active section by where `▸` is and by the detail heading.
- A non-empty watchlist exists only after `p` (pin), which this skill does not press; if the user's machine already has one, the Watchlist step shows `─ Watchlist (N) ─` and `▸` on its first row instead.
