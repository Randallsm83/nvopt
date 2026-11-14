#!/usr/bin/env bash
# Checks nvopt's writes into RTSS profiles and NVIDIA App OPS metadata against the
# properties listed in ../properties.tsv. Reads the nvopt sources, builds and runs only
# under the state directory, and never touches a real RTSS or NVIDIA folder.
#
#   check.sh                 sources: this checkout's src/, else a read-only snapshot of
#                            the hal9k-win working tree
#   check.sh --src DIR       sources: DIR (must hold nvops.rs, rtss.rs, models.rs)
#   check.sh --replay DIR    also replay the injection on a temporary copy of DIR, a real
#                            NvBackend\Recommendations folder (DIR itself is only read)
#   check.sh --record        after an as-expected run, record the source hashes as in sync
#
# Exit status: 0 as expected (holding properties pass, known findings still fail)
#              1 a holding property failed, a property did not run, or the build failed
#              2 a known finding passes now: update properties.tsv and SKILL.md
set -euo pipefail

skill_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_dir="$(cd "$skill_dir/../../.." && pwd)"
state_root="${XDG_STATE_HOME:-$HOME/.local/state}"
scratch="$state_root/formal-nvopt/external-writes"
evidence="$state_root/verify-nvopt/formal-verify-external-writes/runs/$(date -u +%Y%m%dT%H%M%SZ)"
# Files whose behaviour the properties check; their hashes are the coverage record.
covered="nvops.rs rtss.rs"
# Everything the harness compiles: nvops.rs imports NvOpsProfile from models.rs.
needed="$covered models.rs"
win_host="hal9k-win"
win_tree='D:\nvopt'

die() {
    echo "check.sh: $*" >&2
    exit 1
}

src=""
record=0
replay=""
while [ $# -gt 0 ]; do
    case "$1" in
        --src) [ $# -ge 2 ] || die "--src needs a directory"; src="$2"; shift 2 ;;
        --replay) [ $# -ge 2 ] || die "--replay needs a directory"; replay="$2"; shift 2 ;;
        --record) record=1; shift ;;
        -h | --help) sed -n '2,16p' "${BASH_SOURCE[0]}"; exit 0 ;;
        *) die "unknown argument: $1" ;;
    esac
done

command -v cargo > /dev/null 2>&1 ||
    die "cargo is not on PATH (in an agent shell on the Mac: source ~/.config/zsh/agent-env.zsh)"
if [ -n "$replay" ]; then
    [ -d "$replay" ] || die "--replay $replay is not a directory"
    replay="$(cd "$replay" && pwd)"
fi

# One run at a time: the harness is assembled in a shared scratch directory.
mkdir -p "$scratch"
mkdir "$scratch/lock" 2> /dev/null ||
    die "another run holds $scratch/lock (remove that directory if no check.sh is running)"
trap 'rmdir "$scratch/lock"' EXIT

have_needed() {
    for f in $needed; do
        [ -f "$1/$f" ] || return 1
    done
}

# sha256 of a file with carriage returns removed, so CRLF and LF checkouts hash the same.
sha256_lf() {
    if command -v sha256sum > /dev/null 2>&1; then
        tr -d '\r' < "$1" | sha256sum | cut -d' ' -f1
    else
        tr -d '\r' < "$1" | shasum -a 256 | cut -d' ' -f1
    fi
}

# ─── Locate the sources ───
if [ -n "$src" ]; then
    have_needed "$src" || die "--src $src must hold: $needed"
    src="$(cd "$src" && pwd)"
    origin="--src $src"
elif have_needed "$repo_dir/src"; then
    src="$repo_dir/src"
    origin="this checkout ($src)"
else
    snapshot="$scratch/snapshot"
    rm -rf "$snapshot"
    mkdir -p "$snapshot"
    ssh -o BatchMode=yes -o ConnectTimeout=15 "$win_host" \
        "tar -C $win_tree -cf - src/models.rs src/nvops.rs src/rtss.rs Cargo.lock" |
        tar -xf - -C "$snapshot" ||
        die "this checkout has no src/nvops.rs and src/rtss.rs, and the $win_host snapshot failed; pass --src DIR"
    src="$snapshot/src"
    have_needed "$src" || die "the $win_host snapshot lacks one of: $needed"
    origin="read-only snapshot of the $win_host $win_tree working tree"
fi

mkdir -p "$evidence"
summary="$evidence/summary.txt"
say() {
    echo "$*" | tee -a "$summary"
}

say "formal-verify-external-writes  $(date -u +%Y-%m-%dT%H:%M:%SZ)"
say "sources: $origin"

# ─── Compare with the coverage record ───
drift=0
: > "$evidence/source-hashes.txt"
for f in $covered; do
    now="$(sha256_lf "$src/$f")"
    echo "$now  $f" >> "$evidence/source-hashes.txt"
    recorded="$(awk -v f="$f" '$2 == f { print $1 }' "$skill_dir/covered-sources.sha256" 2> /dev/null || true)"
    if [ "$now" != "$recorded" ]; then
        drift=1
        say "DRIFT    $f differs from the coverage record (recorded ${recorded:-nothing}, now $now)"
    fi
done
[ "$drift" -eq 1 ] || say "sources match the coverage record"

# The frame property depends on serde_json's behaviour: warn when nvopt's lock moves off the pin.
product_lock="$(dirname "$src")/Cargo.lock"
if [ -f "$product_lock" ]; then
    pinned="$(sed -n 's/^serde_json = "=\(.*\)"$/\1/p' "$skill_dir/harness/Cargo.toml")"
    locked="$(tr -d '\r' < "$product_lock" | awk '/^name = "serde_json"$/ { getline; gsub(/version = |"/, ""); print; exit }')"
    [ "$pinned" = "$locked" ] ||
        say "WARNING  nvopt locks serde_json $locked but the harness pins $pinned: update harness/Cargo.toml and harness/Cargo.lock"
fi

# ─── Assemble the harness: real sources, verbatim, plus one `mod props;` line each ───
work="$scratch/work"
rm -rf "$work/src" "$work/winreg-stub"
mkdir -p "$work/src/gen"
cp "$skill_dir/harness/Cargo.toml" "$skill_dir/harness/Cargo.lock" "$work/"
cp "$skill_dir/harness/src/lib.rs" "$work/src/lib.rs"
cp -R "$skill_dir/harness/winreg-stub" "$work/winreg-stub"
for f in $needed; do
    tr -d '\r' < "$src/$f" > "$work/src/gen/$f"
done
cp "$skill_dir"/harness/props/*.rs "$work/src/gen/"
printf '\n#[cfg(test)]\n#[path = "nvops_props.rs"]\nmod props;\n' >> "$work/src/gen/nvops.rs"
printf '\n#[cfg(test)]\n#[path = "rtss_props.rs"]\nmod props;\n' >> "$work/src/gen/rtss.rs"

export CARGO_TARGET_DIR="$scratch/target"
# Fixed defaults make a run repeatable; override either variable to explore other inputs.
export PROPTEST_CASES="${PROPTEST_CASES:-512}"
export PROPTEST_RNG_SEED="${PROPTEST_RNG_SEED:-20261113}"
log="$evidence/cargo-test.log"
say "cases per property: $PROPTEST_CASES, seed: $PROPTEST_RNG_SEED"

set +e
(cd "$work" && cargo test --locked --lib -- --test-threads=1) > "$log" 2>&1
set -e
grep -q '^test result:' "$log" || {
    tail -n 30 "$log" >&2
    say "BUILD FAILED  see $log"
    exit 1
}

# ─── Compare the results with properties.tsv ───
results="$(sed -nE 's/^test ([^ ]+) \.\.\. (ok|FAILED)$/\2 \1/p' "$log")"

# The failure proptest reported for one test, in the property's own words (the full
# shrunk input stays in the log).
counterexample() {
    awk -v name="$1" '
        $0 == "---- " name " stdout ----" { in_test = 1; next }
        in_test && (/^---- / || /^failures:$/ || /^minimal failing input/) { exit }
        in_test && /^Test failed: / { show = 1 }
        show { print "         " $0 }
    ' "$log" | sed -n '1,15p'
}

unexpected=0
fixed=0
while IFS="$(printf '\t')" read -r expect name; do
    case "$expect" in '' | \#*) continue ;; esac
    status="$(printf '%s\n' "$results" | awk -v n="$name" '$2 == n { print $1 }')"
    case "$expect:$status" in
        holds:ok) say "PASS     $name" ;;
        holds:FAILED)
            say "BROKEN   $name"
            counterexample "$name" | tee -a "$summary"
            unexpected=1
            ;;
        finding:FAILED)
            say "FINDING  $name (known, still fails)"
            counterexample "$name" | tee -a "$summary"
            ;;
        finding:ok)
            say "FIXED?   $name is listed as a finding but passes now"
            fixed=1
            ;;
        *)
            say "MISSING  $name did not run"
            unexpected=1
            ;;
    esac
done < "$skill_dir/properties.tsv"

# A test that ran without being listed is a property nobody recorded.
while read -r status name; do
    [ -n "$name" ] || continue
    awk -F '\t' -v n="$name" '$2 == n { found = 1 } END { exit !found }' "$skill_dir/properties.tsv" || {
        say "UNLISTED $name ($status) is not in properties.tsv"
        unexpected=1
    }
done << EOF
$results
EOF

# ─── Optional replay on a copy of a real store ───
if [ -n "$replay" ]; then
    replay_log="$evidence/replay.log"
    set +e
    (cd "$work" && NVOPT_REPLAY_RECS="$replay" cargo test --locked --lib -- --ignored --nocapture replay_on_a_copy_of_a_real_store) > "$replay_log" 2>&1
    replay_status=$?
    set -e
    grep -E '^(replay |  outside bracket)' "$replay_log" | tee -a "$summary" || true
    if [ "$replay_status" -ne 0 ]; then
        say "BROKEN   replay on a copy of $replay failed, see $replay_log"
        unexpected=1
    fi
fi

say "evidence: $evidence"
if [ "$unexpected" -ne 0 ]; then
    say "RESULT: FAILED - a property that held is broken, or the run is incomplete"
    exit 1
fi
if [ "$fixed" -ne 0 ]; then
    say "RESULT: a known finding no longer fails - move it to 'holds' in properties.tsv and update SKILL.md"
    exit 2
fi
if [ "$record" -eq 1 ]; then
    {
        echo "# Coverage record for formal-verify-external-writes, written by scripts/check.sh --record."
        echo "# sha256 of each covered file with carriage returns removed."
        echo "# recorded $(date -u +%Y-%m-%dT%H:%MZ) from: $origin"
        cat "$evidence/source-hashes.txt"
    } > "$skill_dir/covered-sources.sha256"
    say "recorded the current source hashes in covered-sources.sha256"
elif [ "$drift" -eq 1 ]; then
    say "NOTE: the covered sources changed since the coverage record. Re-read the coverage table in SKILL.md against them, then run check.sh --record"
fi
say "RESULT: as expected - holding properties pass, known findings still fail"
