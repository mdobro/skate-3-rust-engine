//! Verifies an Android game-data import against `android-manifest.json` (written by
//! `tools/export_android.py`): every listed file must exist with its size and SHA-256.
use crate::sha256::Hasher;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path};

pub const MANIFEST_NAME: &str = "android-manifest.json";

#[derive(Debug, Clone, Deserialize)]
pub struct ImportFile {
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportManifest {
    pub format: u32,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub installation_id: String,
    #[serde(default)]
    pub pipelines: serde_json::Value,
    pub files: BTreeMap<String, ImportFile>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VerifySummary {
    pub files: usize,
    pub bytes: u64,
    pub missing: Vec<String>,
    pub mismatched: Vec<String>,
}

impl VerifySummary {
    pub fn ok(&self) -> bool {
        self.missing.is_empty() && self.mismatched.is_empty()
    }
}

impl ImportManifest {
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(MANIFEST_NAME);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let manifest: Self =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if manifest.format != 1 {
            return Err(format!("Unsupported export format {}", manifest.format));
        }
        Ok(manifest)
    }
}

fn hash_file(path: &Path) -> std::io::Result<(u64, String)> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut size = 0u64;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            return Ok((size, hasher.finish()));
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
}

pub fn verify_installation(root: &Path) -> Result<VerifySummary, String> {
    verify_progress(root, |_, _| {})
}

/// Like `verify_installation`, calling `progress(files_done, total)` after each file.
pub fn verify_progress(
    root: &Path,
    mut progress: impl FnMut(usize, usize),
) -> Result<VerifySummary, String> {
    let manifest = ImportManifest::load(root)?;
    let total = manifest.files.len();
    let mut summary = VerifySummary::default();
    for (done, (rel, want)) in manifest.files.iter().enumerate() {
        let relative = Path::new(rel);
        if relative.is_absolute()
            || relative.components().any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(format!("Unsafe path in manifest: {rel}"));
        }
        match hash_file(&root.join(relative)) {
            Err(_) => summary.missing.push(rel.clone()),
            Ok((size, hash)) => {
                summary.files += 1;
                summary.bytes += size;
                if size != want.size || !hash.eq_ignore_ascii_case(&want.sha256) {
                    summary.mismatched.push(rel.clone());
                }
            }
        }
        progress(done + 1, total);
    }
    Ok(summary)
}

#[cfg(test)]
#[path = "tests/android_import.rs"]
mod tests;
