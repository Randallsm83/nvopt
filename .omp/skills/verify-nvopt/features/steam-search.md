# Steam store search

The user presses `/`, types a game name and presses `Enter`; nvopt queries the Steam Store search API and shows the matching apps in a `Search` section below the installed list, with the first result selected and a result count in the status line.

## Sub-features

- `search-open` `/` replaces the status line with the ` Search: ` input box.
- `search-type` typed characters echo in the box; `Backspace` deletes.
- `search-run` `Enter` runs the query and shows `─ Search (N) ─` with `▸` on the first result and `Found N results for 'Q'`.
- `search-navigate` `j`/`Down` moves the selection inside the results; the detail panel follows.
- `search-cancel` `Esc` closes the box without searching.
- `search-leave` `Tab` from the results returns to the Installed section; results stay listed.

## How to get to it (user POV)

- Press `/` anywhere in normal mode.

## Driving it with drive.py

Preconditions:

- `doctor.ps1` ends with `DOCTOR OK`; hal9k-win can reach `store.steampowered.com`.

- **Open.** Run `drive.py 'wait:─ Installed (' snap:launched key:/ 'wait: Search: '`. The bottom line shows ` Search: █`.
- **Type.** Add `type:Cyberpunk snap:query-typed`. The box reads ` Search: Cyberpunk█`.
- **Run.** Add `key:enter 'wait:─ Search (' 'wait:results for' snap:results`. The list ends with `─ Search (10) ─`, `▸ Cyberpunk 2077 ○` is the first row, the detail panel heading is `Cyberpunk 2077`, the status reads ` Found 10 results for 'Cyberpunk'`.
- **Navigate.** Add `key:down snap:second-result`. `▸` moves to the second result (`Probably Stolen - Cyberpunk Shopkeeper Sim` at last proof) and the detail heading changes to it.
- **Leave.** Add `key:tab 'wait:Installed' snap:after-tab`. `▸` is back on the first installed game (`3DMark`); the `Search` rows remain.
- **Cancel.** In a separate drive: `key:/ 'wait: Search: ' type:abc key:esc 'gone: Search: '`. The status line returns unchanged and no `─ Search (` header appears.
- **Proof.** `03-results.txt` and `04-second-result.txt` show the query result and the selection move; `sidefx-diff.txt` is empty.

Full proven command:

```sh
$S/drive.py 'wait:─ Installed (' snap:launched key:/ 'wait: Search: ' type:Cyberpunk snap:query-typed \
  key:enter 'wait:─ Search (' 'wait:results for' snap:results key:down snap:second-result key:tab 'wait:Installed' snap:after-tab
```

## Gotchas

- The count and order come from the live Steam Store and change over time; assert `Found N results for 'Cyberpunk'` with N > 0 and `Cyberpunk 2077` present, not the exact list.
- A query with no matches sets `Found 0 results` and shows no `─ Search (` header at all; an empty query does the same without a network call.
- Network failure shows ` Search error: …` and leaves the previous results in place.
- `Enter` on a search result (normal mode) would fetch PCGW data and write the cache; `p` would pin it to `watchlist.toml`. Both are off limits here.
- Search results never show the `◆` OPS icon until a fetch, which this skill does not do.
