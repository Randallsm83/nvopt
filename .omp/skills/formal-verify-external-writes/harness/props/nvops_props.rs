//! Properties of nvopt's writes into the NVIDIA App's OPS metadata.
//!
//! This file is compiled as a child module of the real `nvops.rs`, so `super::` is the
//! product code: `NvOpsIndex::inject_resolution` (the `I` key), `inject_into_ops_array`,
//! `interpolate_pops_1based` and `pick_pops_for_resolution`.

use super::*;
use proptest::prelude::*;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

const VARIANTS: [&str; 2] = ["regular_rtx", "regular"];

/// Every resolution found in real ops arrays (27 game folders on hal9k-win, 2026-11-13).
const SHIPPED: [(u32, u32); 14] = [
    (1024, 768),
    (1280, 1024),
    (1360, 768),
    (1600, 900),
    (1680, 1050),
    (1920, 1080),
    (1920, 1200),
    (2560, 1080),
    (2560, 1440),
    (2560, 1600),
    (3440, 1440),
    (3840, 2160),
    (3840, 2400),
    (7680, 4320),
];

fn config() -> ProptestConfig {
    ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

// ─── Generated inputs ───

/// One ops entry in the shape NVIDIA writes: resolution, pops, rate, belowMinSpec.
#[derive(Debug, Clone)]
struct Entry {
    res: (u32, u32),
    pops: usize,
    rate: u32,
}

fn res_str(res: (u32, u32)) -> String {
    format!("{}x{}", res.0, res.1)
}

fn pixels(res: (u32, u32)) -> u64 {
    res.0 as u64 * res.1 as u64
}

fn entry_json(e: &Entry) -> Value {
    json!({"resolution": res_str(e.res), "pops": e.pops, "rate": e.rate, "belowMinSpec": false})
}

fn describe(entries: &[Entry]) -> String {
    entries
        .iter()
        .map(|e| format!("{}→pops {}", res_str(e.res), e.pops))
        .collect::<Vec<_>>()
        .join(", ")
}

fn resolution() -> impl Strategy<Value = (u32, u32)> {
    prop_oneof![
        3 => proptest::sample::select(SHIPPED.to_vec()),
        1 => (320u32..=8192, 200u32..=4320),
    ]
}

/// 1 to 11 entries with distinct resolutions, in no particular order (real arrays are unsorted;
/// the longest real one has 11 entries and real pops run from 1 to 56).
fn entries() -> impl Strategy<Value = Vec<Entry>> {
    proptest::collection::btree_map(
        resolution(),
        (1usize..=56, prop_oneof![Just(40u32), Just(60u32)]),
        1..=11,
    )
    .prop_map(|m| {
        m.into_iter()
            .map(|(res, (pops, rate))| Entry { res, pops, rate })
            .collect::<Vec<_>>()
    })
    .prop_shuffle()
}

/// A display resolution to look up or inject: a common one, a rotated copy of an existing
/// entry (same pixel count, different text), or one below or above every entry.
fn entries_and_target() -> impl Strategy<Value = (Vec<Entry>, (u32, u32))> {
    entries().prop_flat_map(|es| {
        let rotated: Vec<(u32, u32)> = es.iter().map(|e| (e.res.1, e.res.0)).collect();
        let target = prop_oneof![
            2 => resolution(),
            2 => proptest::sample::select(rotated),
            1 => (1u32..320, 1u32..200),
            1 => (8193u32..=16000, 4321u32..=9000),
        ];
        (Just(es), target)
    })
}

/// Which numbers the generated unknown JSON may hold.
#[derive(Debug, Clone, Copy)]
enum Numbers {
    /// 64-bit integers only. Every number in the real files is an integer below 2^53.
    Integers,
    /// Integers and fractional numbers, which the real files do not hold today.
    WithFractions,
}

/// Arbitrary JSON that nvopt does not know about.
fn unknown_json(numbers: Numbers) -> BoxedStrategy<Value> {
    let integer = prop_oneof![any::<i64>().prop_map(Value::from), any::<u64>().prop_map(Value::from)];
    let number = match numbers {
        Numbers::Integers => integer.boxed(),
        Numbers::WithFractions => prop_oneof![
            1 => integer,
            3 => any::<f64>().prop_filter("finite", |f| f.is_finite()).prop_map(Value::from),
        ]
        .boxed(),
    };
    let leaf = prop_oneof![
        1 => Just(Value::Null),
        1 => any::<bool>().prop_map(Value::from),
        3 => number,
        1 => "[ -~]{0,12}".prop_map(Value::from),
        1 => "\\PC{0,6}".prop_map(Value::from),
    ];
    leaf.prop_recursive(3, 16, 4, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
            proptest::collection::btree_map("[A-Za-z_]{1,8}", inner, 0..4)
                .prop_map(|m| Value::Object(m.into_iter().collect())),
        ]
    })
    .boxed()
}

fn unknown_fields(numbers: Numbers) -> impl Strategy<Value = Map<String, Value>> {
    proptest::collection::btree_map("[A-Za-z_]{1,8}", unknown_json(numbers), 0..3)
        .prop_map(|m| m.into_iter().collect())
}

/// The value under `ops`: usually a well-formed array, sometimes one with extra fields or
/// junk elements, sometimes empty, sometimes not an array at all.
fn ops_value(numbers: Numbers) -> impl Strategy<Value = Value> {
    prop_oneof![
        6 => entries().prop_map(|es| Value::Array(es.iter().map(entry_json).collect())),
        2 => (entries(), proptest::collection::vec(unknown_json(numbers), 0..3), unknown_fields(numbers)).prop_map(
            |(es, junk, extra)| {
                let mut items: Vec<Value> = es
                    .iter()
                    .map(|e| {
                        let mut obj = extra.clone();
                        obj.extend(entry_json(e).as_object().unwrap().clone());
                        Value::Object(obj)
                    })
                    .collect();
                items.extend(junk);
                Value::Array(items)
            }
        ),
        1 => Just(json!([])),
        1 => unknown_json(numbers),
    ]
}

/// A `regular` / `regular_rtx` block: version, pops (the download record), ops, plus unknowns.
fn variant_block(numbers: Numbers) -> impl Strategy<Value = Value> {
    (ops_value(numbers), "[0-9]{1,10}\\.[0-9]{1,10}\\.[0-9]{1,10}", unknown_fields(numbers)).prop_map(
        |(ops, version, mut block)| {
            block.insert("version".into(), Value::from(version));
            block.insert(
                "pops".into(),
                json!({"filename": "pops.pub.tsv", "url": "https://example.invalid/ops.json", "sha256": "00", "size": 27503}),
            );
            block.insert("ops".into(), ops);
            Value::Object(block)
        },
    )
}

/// One game folder as the NVIDIA App lays it out:
/// `<id>/profiles_metadata.json`, `<id>/<hash>/metadata.json`, `<id>/<hash>/<variant>/metadata.json`.
#[derive(Debug, Clone)]
struct GameDir {
    hash: String,
    top: Value,
    nested: BTreeMap<&'static str, Value>,
}

fn game_dir(numbers: Numbers) -> impl Strategy<Value = GameDir> {
    (
        "[0-9a-f]{8}",
        proptest::collection::btree_map(proptest::sample::select(VARIANTS.to_vec()), variant_block(numbers), 0..=2),
        proptest::collection::btree_map(proptest::sample::select(VARIANTS.to_vec()), variant_block(numbers), 0..=2),
        any::<bool>(),
        "[0-9a-f]{0,16}",
        unknown_fields(numbers),
    )
        .prop_map(|(hash, blocks, other_blocks, mirror, etag, mut top)| {
            top.insert("etag".into(), Value::from(etag));
            for (variant, block) in &blocks {
                top.insert((*variant).into(), block.clone());
            }
            // Real nested files mirror the top-level block; also try ones that differ.
            let nested_blocks = if mirror { blocks } else { other_blocks };
            let nested = nested_blocks
                .into_iter()
                .map(|(variant, block)| (variant, json!({"metadata": {variant: block}})))
                .collect();
            GameDir { hash, top: Value::Object(top), nested }
        })
}

fn games(numbers: Numbers) -> impl Strategy<Value = BTreeMap<String, GameDir>> {
    proptest::collection::btree_map("[a-z0-9_]{1,12}", game_dir(numbers), 1..=3)
}

fn write_store(root: &Path, games: &BTreeMap<String, GameDir>) -> NvOpsIndex {
    let mut index = HashMap::new();
    for (id, game) in games {
        let game_dir = root.join(id);
        let hash_dir = game_dir.join(&game.hash);
        fs::create_dir_all(&hash_dir).unwrap();
        fs::write(
            game_dir.join("profiles_metadata.json"),
            json!({"ce": game.hash, "cv": 1, "ue": game.hash, "uv": 1}).to_string(),
        )
        .unwrap();
        fs::write(hash_dir.join("metadata.json"), game.top.to_string()).unwrap();
        for (variant, doc) in &game.nested {
            fs::create_dir_all(hash_dir.join(variant)).unwrap();
            fs::write(hash_dir.join(variant).join("metadata.json"), doc.to_string()).unwrap();
        }
        index.insert(id.clone(), game_dir);
    }
    NvOpsIndex { games: index }
}

/// Every file under `root`, by relative path.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                out.insert(path.strip_prefix(root).unwrap().to_path_buf(), fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

// ─── What an injection is allowed to do ───

/// True when nvopt can read this element as an ops entry (`OpsEntry` in nvops.rs).
fn usable_entry(v: &Value) -> bool {
    let Some(obj) = v.as_object() else { return false };
    let optional = |key: &str, ok: &dyn Fn(&Value) -> bool| obj.get(key).map_or(true, |v| v.is_null() || ok(v));
    obj.get("resolution").map_or(false, Value::is_string)
        && obj.get("pops").map_or(false, Value::is_u64)
        && optional("rate", &|v| v.as_u64().map_or(false, |n| n <= u32::MAX as u64))
        && optional("belowMinSpec", &Value::is_boolean)
}

fn has_resolution(ops: &[Value], res: &str) -> bool {
    ops.iter().any(|e| e.get("resolution").and_then(Value::as_str) == Some(res))
}

/// One ops array before and after: either untouched, or exactly one new entry in front of
/// the old ones. Returns whether an entry was added.
fn check_ops(before: &Value, after: &Value, res: &str) -> Result<bool, String> {
    let expect_added = before
        .as_array()
        .map_or(false, |ops| !has_resolution(ops, res) && ops.iter().any(usable_entry));
    if !expect_added {
        return if before == after {
            Ok(false)
        } else {
            Err(format!("ops changed although {res} was already there or nothing could be read: {before} became {after}"))
        };
    }
    let (b, a) = (before.as_array().unwrap(), after.as_array().ok_or("ops is no longer an array")?);
    if a.len() != b.len() + 1 || a[1..] != b[..] {
        return Err(format!("expected one new entry in front of the old ones: {before} became {after}"));
    }
    let new = a[0].as_object().ok_or("the new entry is not an object")?;
    let keys: BTreeSet<&str> = new.keys().map(String::as_str).collect();
    let well_formed = keys == BTreeSet::from(["belowMinSpec", "pops", "rate", "resolution"])
        && new["resolution"] == res
        && new["belowMinSpec"] == false
        && new["pops"].as_u64().map_or(false, |p| p >= 1)
        && new["rate"].is_u64();
    if well_formed {
        Ok(true)
    } else {
        Err(format!("the new entry is malformed: {}", a[0]))
    }
}

/// A whole document before and after: nothing may differ except the ops arrays at
/// `ops_paths`, each of which must pass `check_ops`. Returns how many arrays gained an entry.
fn check_doc(before: &Value, after: &Value, ops_paths: &[Vec<&str>], res: &str) -> Result<usize, String> {
    fn at<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
        path.iter().try_fold(v, |v, key| v.get(*key))
    }
    let mut restored = after.clone();
    let mut added = 0;
    for path in ops_paths {
        match (at(before, path), at(after, path)) {
            (None, None) => {}
            (Some(b), Some(a)) => {
                if check_ops(b, a, res)? {
                    added += 1;
                    let slot = path.iter().try_fold(&mut restored, |v, key| v.get_mut(*key)).unwrap();
                    *slot = b.clone();
                }
            }
            _ => return Err(format!("{} appeared or vanished", path.join("."))),
        }
    }
    match first_difference(before, &restored, "") {
        None => Ok(added),
        Some(diff) => Err(format!("something outside the ops arrays changed: {diff}")),
    }
}

/// Where two documents first differ, as "<path>: <old> became <new>".
fn first_difference(old: &Value, new: &Value, path: &str) -> Option<String> {
    match (old, new) {
        (Value::Object(a), Value::Object(b)) if a.len() == b.len() && a.keys().eq(b.keys()) => a
            .iter()
            .find_map(|(key, value)| first_difference(value, &b[key], &format!("{path}.{key}"))),
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a
            .iter()
            .zip(b)
            .enumerate()
            .find_map(|(i, (x, y))| first_difference(x, y, &format!("{path}[{i}]"))),
        _ if old == new => None,
        _ => Some(format!("{}: {old} became {new}", path.trim_start_matches('.'))),
    }
}

/// The ops arrays nvopt may touch in a file, from its place in the game folder:
/// `<id>/<hash>/metadata.json` or `<id>/<hash>/<variant>/metadata.json`. None for other files.
fn ops_paths_for(rel: &Path) -> Option<Vec<Vec<&'static str>>> {
    let parts: Vec<&str> = rel.iter().map(|p| p.to_str().unwrap()).collect();
    match parts.as_slice() {
        [_, _, "metadata.json"] => Some(VARIANTS.iter().map(|v| vec![*v, "ops"]).collect()),
        [_, _, variant, "metadata.json"] => VARIANTS
            .iter()
            .copied()
            .find(|v| v == variant)
            .map(|v| vec![vec!["metadata", v, "ops"]]),
        _ => None,
    }
}

/// Compare two snapshots of a store. Returns the game ids whose top-level metadata.json
/// gained an entry.
fn check_store(
    before: &BTreeMap<PathBuf, Vec<u8>>,
    after: &BTreeMap<PathBuf, Vec<u8>>,
    res: &str,
) -> Result<BTreeSet<String>, String> {
    if before.keys().ne(after.keys()) {
        return Err("files were created or removed".into());
    }
    let mut patched_games = BTreeSet::new();
    for (rel, old) in before {
        let new = &after[rel];
        let Some(paths) = ops_paths_for(rel) else {
            if old != new {
                return Err(format!("{} is not a metadata.json nvopt may patch, but it changed", rel.display()));
            }
            continue;
        };
        let (Ok(old_doc), Ok(new_doc)) = (serde_json::from_slice::<Value>(old), serde_json::from_slice::<Value>(new)) else {
            if old != new {
                return Err(format!("{} is not JSON, but it changed", rel.display()));
            }
            continue;
        };
        let added = check_doc(&old_doc, &new_doc, &paths, res).map_err(|e| format!("{}: {e}", rel.display()))?;
        if added == 0 && old != new {
            return Err(format!("{} was rewritten although nothing was injected", rel.display()));
        }
        if added > 0 && rel.iter().count() == 3 {
            patched_games.insert(rel.iter().next().unwrap().to_str().unwrap().to_string());
        }
    }
    Ok(patched_games)
}

/// The pops an interpolation by pixel count may give `target`: anything from the lowest to
/// the highest pops among the nearest entries at or below and at or above its pixel count.
fn bracket(entries: &[Entry], target: (u32, u32)) -> (usize, usize) {
    let t = pixels(target);
    let lower = entries.iter().map(|e| pixels(e.res)).filter(|p| *p <= t).max();
    let upper = entries.iter().map(|e| pixels(e.res)).filter(|p| *p >= t).min();
    let nearest: Vec<usize> = entries
        .iter()
        .filter(|e| Some(pixels(e.res)) == lower || Some(pixels(e.res)) == upper)
        .map(|e| e.pops)
        .collect();
    (*nearest.iter().min().unwrap(), *nearest.iter().max().unwrap())
}

// ─── Properties ───

/// Shared body: inject `target` into a generated store and require that only the new
/// entries changed and that the reported counts match what happened on disk.
fn only_the_new_entry_changes(games: &BTreeMap<String, GameDir>, target: (u32, u32)) -> Result<(), TestCaseError> {
    let root = tempfile::tempdir().unwrap();
    let index = write_store(root.path(), games);
    let before = snapshot(root.path());

    let result = index.inject_resolution(target);

    let after = snapshot(root.path());
    let patched = check_store(&before, &after, &res_str(target));
    prop_assert!(patched.is_ok(), "{}", patched.unwrap_err());
    prop_assert_eq!(result.patched, patched.unwrap().len(), "the reported patched count is wrong");
    prop_assert_eq!(result.patched + result.skipped_existing + result.errors.len(), games.len());
    Ok(())
}

proptest! {
    #![proptest_config(config())]

    /// Pressing `I` adds one entry for the display resolution in front of each ops array that
    /// lacks it, and changes nothing else in the NVIDIA App's files (files whose numbers are
    /// integers, as NVIDIA ships them).
    #[test]
    fn inject_changes_only_the_new_entry(games in games(Numbers::Integers), target in resolution()) {
        only_the_new_entry_changes(&games, target)?;
    }

    /// The same, for files that also hold fractional numbers: those numbers stay exactly as
    /// they were.
    #[test]
    fn inject_keeps_fractional_numbers(games in games(Numbers::WithFractions), target in resolution()) {
        only_the_new_entry_changes(&games, target)?;
    }

    /// Pressing `I` a second time changes no byte and reports nothing patched.
    #[test]
    fn inject_twice_is_inject_once(games in games(Numbers::Integers), target in resolution()) {
        let root = tempfile::tempdir().unwrap();
        let index = write_store(root.path(), &games);
        index.inject_resolution(target);
        let once = snapshot(root.path());

        let second = index.inject_resolution(target);

        prop_assert!(second.patched == 0, "the second injection reported {} games patched", second.patched);
        prop_assert!(snapshot(root.path()) == once, "the second injection changed a file");
    }

    /// The pops written for an injected resolution lies between the pops of the nearest
    /// existing resolutions by pixel count.
    #[test]
    fn injected_pops_within_bracket((entries, target) in entries_and_target()) {
        let mut ops: Vec<Value> = entries.iter().map(entry_json).collect();

        // Whether an entry is added at all belongs to inject_changes_only_the_new_entry.
        if inject_into_ops_array(&mut ops, &res_str(target)).unwrap() {
            let pops = ops[0]["pops"].as_u64().unwrap() as usize;
            let (lo, hi) = bracket(&entries, target);
            prop_assert!(
                lo <= pops && pops <= hi,
                "injecting {} wrote pops {}, but the nearest entries by pixel count have pops {}..={} (ops: {})",
                res_str(target), pops, lo, hi, describe(&entries)
            );
        }
    }

    /// The quality tier nvopt picks for a display resolution is a valid tier index, and a
    /// resolution NVIDIA lists gets exactly the tier NVIDIA gave it.
    #[test]
    fn picked_tier_is_valid_and_listed_resolution_wins((entries, target) in entries_and_target(), tier_count in 1usize..=64) {
        let (picked, _source, _rate) = pick_pops_for_resolution(&ops_entries(&entries), target, tier_count);

        prop_assert!(picked < tier_count, "picked tier index {} but there are only {} tiers", picked, tier_count);
        if let Some(listed) = entries.iter().find(|e| e.res == target) {
            let want = tier_index(listed.pops, tier_count);
            prop_assert!(
                picked == want,
                "{} is listed with pops {} (tier index {}), but nvopt picked tier index {}",
                res_str(target), listed.pops, want, picked
            );
        }
    }

    /// For a resolution NVIDIA does not list, the tier nvopt picks lies between the tiers of
    /// the nearest listed resolutions by pixel count.
    #[test]
    fn picked_tier_within_bracket((entries, target) in entries_and_target(), tier_count in 1usize..=64) {
        let (picked, _source, _rate) = pick_pops_for_resolution(&ops_entries(&entries), target, tier_count);

        if entries.iter().all(|e| e.res != target) {
            let (lo, hi) = bracket(&entries, target);
            let (lo, hi) = (tier_index(lo, tier_count), tier_index(hi, tier_count));
            prop_assert!(
                lo <= picked && picked <= hi,
                "for {} nvopt picked tier index {}, but the nearest entries by pixel count give {}..={} (ops: {})",
                res_str(target), picked, lo, hi, describe(&entries)
            );
        }
    }
}

fn ops_entries(entries: &[Entry]) -> Vec<OpsEntry> {
    entries
        .iter()
        .map(|e| OpsEntry { resolution: res_str(e.res), pops: e.pops, rate: Some(e.rate), below_min_spec: Some(false) })
        .collect()
}

/// pops is 1-based in NVIDIA's files; nvopt keeps a 0-based index into its tiers.
fn tier_index(pops: usize, tier_count: usize) -> usize {
    pops.saturating_sub(1).min(tier_count - 1)
}

// ─── Replay on a copy of a real store (run by `check.sh --replay <dir>`) ───

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_tree(&path, &dest);
        } else {
            fs::copy(&path, &dest).unwrap();
        }
    }
}

/// Copies a real `NvBackend\Recommendations` folder (NVOPT_REPLAY_RECS) into a temporary
/// directory and injects there. The folder named by the variable is only read.
#[test]
#[ignore = "needs NVOPT_REPLAY_RECS; run through check.sh --replay <dir>"]
fn replay_on_a_copy_of_a_real_store() {
    let source = PathBuf::from(std::env::var("NVOPT_REPLAY_RECS").expect("NVOPT_REPLAY_RECS is not set"));
    // A resolution NVIDIA does not ship (the example in config.rs) and a rotated 4K display.
    for target in [(7680u32, 2160u32), (2160, 3840)] {
        let root = tempfile::tempdir().unwrap();
        copy_tree(&source, root.path());
        let games: HashMap<String, PathBuf> = fs::read_dir(root.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir())
            .map(|p| (p.file_name().unwrap().to_string_lossy().to_string(), p))
            .collect();
        assert!(!games.is_empty(), "no game folders under {}", source.display());
        let index = NvOpsIndex { games };
        let res = res_str(target);
        let before = snapshot(root.path());

        let result = index.inject_resolution(target);

        let after = snapshot(root.path());
        let patched = check_store(&before, &after, &res).unwrap_or_else(|e| panic!("replay {res}: {e}"));
        assert_eq!(result.patched, patched.len(), "replay {res}: reported patched count is wrong");
        let second = index.inject_resolution(target);
        assert_eq!(second.patched, 0, "replay {res}: second injection patched again");
        assert!(snapshot(root.path()) == after, "replay {res}: second injection changed a file");

        // How the bracket property fares on the real arrays (reported, not asserted here).
        let mut outside = Vec::new();
        for (rel, old) in &before {
            if rel.iter().count() != 3 || rel.file_name().unwrap() != "metadata.json" {
                continue;
            }
            let (Ok(old_doc), Ok(new_doc)) =
                (serde_json::from_slice::<Value>(old), serde_json::from_slice::<Value>(&after[rel]))
            else {
                continue;
            };
            for variant in VARIANTS {
                let (Some(old_ops), Some(new_ops)) = (
                    old_doc.get(variant).and_then(|b| b.get("ops")).and_then(Value::as_array),
                    new_doc.get(variant).and_then(|b| b.get("ops")).and_then(Value::as_array),
                ) else { continue };
                if new_ops.len() != old_ops.len() + 1 {
                    continue;
                }
                let entries: Vec<Entry> = old_ops
                    .iter()
                    .filter_map(|e| {
                        let res = parse_resolution(e.get("resolution")?.as_str()?)?;
                        Some(Entry { res, pops: e.get("pops")?.as_u64()? as usize, rate: 0 })
                    })
                    .collect();
                if entries.is_empty() {
                    continue;
                }
                let pops = new_ops[0]["pops"].as_u64().unwrap() as usize;
                let (lo, hi) = bracket(&entries, target);
                if pops < lo || pops > hi {
                    outside.push(format!("{} wrote {pops}, nearest entries have {lo}..={hi}", rel.iter().next().unwrap().to_string_lossy()));
                }
            }
        }
        println!(
            "replay {res}: games={} patched={} already-had-it={} errors={} only-new-entry=held second-run-no-change=held pops-outside-bracket={}",
            index.games.len(), result.patched, result.skipped_existing, result.errors.len(), outside.len()
        );
        for line in &outside {
            println!("  outside bracket: {line}");
        }
    }
}
