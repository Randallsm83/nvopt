//! Properties of nvopt's writes into RTSS application profiles.
//!
//! This file is compiled as a child module of the real `rtss.rs`, so `super::` is the
//! product code: `RtssManager::set_fps_limit` (the `t` key), `remove_fps_limit` (the `T`
//! key), `parse_profile` and `save_profile`.

use super::*;
use proptest::prelude::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn config() -> ProptestConfig {
    ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

/// An RTSS install rooted in a temporary directory. `RtssManager::new` is never called, so
/// no real RTSS folder or registry key is looked up.
fn manager(root: &Path) -> RtssManager {
    RtssManager {
        install_path: Some(root.to_path_buf()),
        profiles_path: Some(root.join("Profiles")),
    }
}

/// Every file under `root`, by relative path.
fn files(root: &Path) -> BTreeSet<PathBuf> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeSet<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                out.insert(path.strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(root, root, &mut out);
    out
}

/// Executable names of the shape `App::game_exe` (app.rs) passes in: the letters and digits
/// of the game's name, then ".exe".
fn game_exe() -> impl Strategy<Value = String> {
    "[\\p{L}\\p{N}]{1,24}".prop_map(|stem| format!("{stem}.exe"))
}

/// Any text at all as an executable name, weighted towards path tricks.
fn hostile_exe() -> impl Strategy<Value = String> {
    prop_oneof![
        2 => "\\PC{0,24}",
        2 => "(\\.\\./|\\.\\.\\\\){1,3}[a-z]{1,8}(\\.exe)?",
        1 => "[a-z]{0,5}[/\\\\:][a-z.]{0,8}",
        1 => "[A-Z]:[/\\\\][a-z]{1,8}\\.exe",
    ]
}

/// The path `set_fps_limit` uses for a `game_exe()` name.
fn profile_path(root: &Path, exe: &str) -> PathBuf {
    root.join("Profiles").join(format!("{}.cfg", exe.trim_end_matches(".exe")))
}

/// Keys nvopt reads and re-emits itself; every other key belongs to RTSS or the user.
const MODELLED_KEYS: [&str; 8] = [
    "AppName",
    "Application",
    "FramerateLimit",
    "FPSLimit",
    "ScanlineSync",
    "EnableOSD",
    "ShowFPS",
    "ShowFrametime",
];

/// Lines of an existing profile: section headers and `Key=Value` settings.
fn existing_profile_lines() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec(
        prop_oneof![
            1 => "\\[[A-Za-z0-9]{1,12}\\]",
            4 => ("[A-Z][A-Za-z0-9]{0,11}", "[!-~]{0,12}").prop_map(|(key, value)| format!("{key}={value}")),
            1 => (proptest::sample::select(MODELLED_KEYS.to_vec()), "[0-9]{1,3}").prop_map(|(key, value)| format!("{key}={value}")),
        ],
        1..12,
    )
}

proptest! {
    #![proptest_config(config())]

    /// Whatever the executable name, setting a cap writes one `.cfg` file directly inside
    /// RTSS's Profiles folder and nothing anywhere else.
    #[test]
    fn set_fps_limit_stays_inside_profiles_dir(exe in hostile_exe(), cap in any::<u32>()) {
        let root = tempfile::tempdir().unwrap();
        let install = root.path().join("rtss");
        fs::create_dir_all(install.join("Profiles")).unwrap();
        fs::write(root.path().join("outside.txt"), "untouched").unwrap();
        fs::write(install.join("RTSS.exe"), "untouched").unwrap();
        let before = files(root.path());

        // An error is fine (the name may not be a legal file name); a stray file is not.
        let _ = manager(&install).set_fps_limit(&exe, cap);

        let created: Vec<PathBuf> = files(root.path()).difference(&before).cloned().collect();
        prop_assert!(created.len() <= 1, "more than one file created for {:?}: {:?}", exe, created);
        for rel in &created {
            let name = rel.file_name().unwrap().to_string_lossy().to_string();
            prop_assert!(
                rel.parent() == Some(Path::new("rtss/Profiles")),
                "set_fps_limit({:?}, ..) wrote outside the Profiles folder: {:?}", exe, rel
            );
            prop_assert!(
                name.ends_with(".cfg") && !name.contains(['/', '\\', ':']),
                "{:?} produced the file name {:?}", exe, name
            );
        }
        let untouched = |path: PathBuf| fs::read_to_string(path).unwrap() == "untouched";
        prop_assert!(
            untouched(root.path().join("outside.txt")) && untouched(install.join("RTSS.exe")),
            "set_fps_limit({:?}, ..) overwrote a file outside the Profiles folder", exe
        );
    }

    /// On a profile nvopt wrote itself, setting a cap changes the cap and nothing else,
    /// setting it again changes no byte, and clearing it leaves the profile unlimited.
    #[test]
    fn cap_roundtrip_on_nvopt_written_profile(
        exe in game_exe(),
        old_limit in proptest::option::of(any::<u32>()),
        scanline_sync in proptest::option::of(any::<i32>()),
        flags in any::<(bool, bool, bool)>(),
        cap in any::<u32>(),
    ) {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("Profiles")).unwrap();
        let path = profile_path(root.path(), &exe);
        let written = RtssProfile {
            name: exe.trim_end_matches(".exe").to_string(),
            executable: exe.clone(),
            framerate_limit: old_limit,
            scanline_sync,
            osd_enabled: flags.0,
            show_fps: flags.1,
            show_frametime: flags.2,
        };
        save_profile(&path, &written).unwrap();
        let rtss = manager(root.path());
        let rest = |p: &RtssProfile| (p.name.clone(), p.executable.clone(), p.scanline_sync, p.osd_enabled, p.show_fps, p.show_frametime);

        rtss.set_fps_limit(&exe, cap).unwrap();
        let capped = parse_profile(&path).unwrap();
        let bytes = fs::read(&path).unwrap();
        prop_assert!(capped.framerate_limit == Some(cap), "after set_fps_limit(.., {}) the profile's cap reads {:?}", cap, capped.framerate_limit);
        prop_assert!(rest(&capped) == rest(&written), "setting the cap changed another field: {:?} became {:?}", rest(&written), rest(&capped));

        rtss.set_fps_limit(&exe, cap).unwrap();
        prop_assert!(fs::read(&path).unwrap() == bytes, "setting the same cap twice changed the file");

        rtss.remove_fps_limit(&exe).unwrap();
        let cleared = parse_profile(&path).unwrap();
        prop_assert!(
            cleared.framerate_limit == Some(0) && cleared.fps_display() == "Unlimited",
            "after remove_fps_limit the profile's cap reads {:?}", cleared.framerate_limit
        );
        prop_assert!(rest(&cleared) == rest(&written), "clearing the cap changed another field: {:?} became {:?}", rest(&written), rest(&cleared));
        prop_assert!(files(root.path()).len() == 1, "more than the one profile file exists: {:?}", files(root.path()));
    }

    /// Setting a cap on a profile that already exists keeps every setting nvopt does not
    /// model.
    #[test]
    fn set_fps_limit_keeps_foreign_keys(exe in game_exe(), lines in existing_profile_lines(), cap in 1u32..=1000) {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("Profiles")).unwrap();
        let path = profile_path(root.path(), &exe);
        fs::write(&path, lines.join("\n") + "\n").unwrap();

        manager(root.path()).set_fps_limit(&exe, cap).unwrap();

        let now = fs::read_to_string(&path).unwrap();
        for line in &lines {
            let Some((key, _)) = line.split_once('=') else { continue };
            if MODELLED_KEYS.contains(&key) {
                continue;
            }
            prop_assert!(
                now.lines().any(|l| l.trim() == line),
                "the existing profile line {:?} is gone after set_fps_limit({:?}, {}); the file is now:\n{}",
                line, exe, cap, now
            );
        }
    }
}
