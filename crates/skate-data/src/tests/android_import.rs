use super::*;
use std::path::PathBuf;

const HELLO: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("skate-android-import-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("maps")).unwrap();
    dir
}

fn write(root: &Path, hash: &str, size: u64) {
    std::fs::write(root.join("maps/a.skate"), b"hello").unwrap();
    std::fs::write(
        root.join(MANIFEST_NAME),
        format!(r#"{{"format":1,"files":{{"maps/a.skate":{{"size":{size},"sha256":"{hash}"}}}}}}"#),
    )
    .unwrap();
}

#[test]
fn streaming_hash_matches_one_shot_across_block_boundaries() {
    let data: Vec<u8> = (0..1000u32).map(|i| (i * 7) as u8).collect();
    for step in [1, 13, 63, 64, 65, 300] {
        let mut h = Hasher::new();
        data.chunks(step).for_each(|c| h.update(c));
        assert_eq!(h.finish(), crate::sha256::digest(&data), "step {step}");
    }
}

#[test]
fn accepts_matching_files_and_reports_progress() {
    let root = temp("ok");
    write(&root, HELLO, 5);
    let mut calls = vec![];
    let summary = verify_progress(&root, |d, t| calls.push((d, t))).unwrap();
    assert!(summary.ok());
    assert_eq!((summary.files, summary.bytes), (1, 5));
    assert_eq!(calls, [(1, 1)]);
}

#[test]
fn reports_mismatched_and_missing_files() {
    let root = temp("bad");
    write(&root, &"0".repeat(64), 5);
    assert_eq!(verify_installation(&root).unwrap().mismatched, ["maps/a.skate"]);
    write(&root, HELLO, 6);
    assert_eq!(verify_installation(&root).unwrap().mismatched, ["maps/a.skate"]);
    std::fs::remove_file(root.join("maps/a.skate")).unwrap();
    assert_eq!(verify_installation(&root).unwrap().missing, ["maps/a.skate"]);
}

#[test]
fn rejects_missing_manifest_unknown_format_and_escaping_paths() {
    let root = temp("manifest");
    assert!(verify_installation(&root).is_err());
    std::fs::write(root.join(MANIFEST_NAME), r#"{"format":2,"files":{}}"#).unwrap();
    assert!(verify_installation(&root).is_err());
    std::fs::write(
        root.join(MANIFEST_NAME),
        format!(r#"{{"format":1,"files":{{"../x":{{"size":0,"sha256":"{HELLO}"}}}}}}"#),
    )
    .unwrap();
    assert!(verify_installation(&root).unwrap_err().contains("Unsafe"));
}
