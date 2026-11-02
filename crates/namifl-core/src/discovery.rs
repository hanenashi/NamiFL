use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::domain::PhotoGroup;

const RAW_EXTENSIONS: &[&str] = &[
    "pef", "dng", "nef", "cr2", "cr3", "arw", "orf", "rw2", "raf",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScanReport {
    pub groups: Vec<PhotoGroup>,
    /// Recognized files whose stems cannot be represented as UTF-8.
    pub unsupported_names: Vec<PathBuf>,
}

/// Non-recursive discovery. Does not follow symlinks or mutate source files.
/// Extension matching is case-insensitive; stem matching is exact.
pub fn scan_folder(folder: &Path) -> io::Result<ScanReport> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(group_paths(paths))
}

/// Pure grouping helper; callers are responsible for supplying regular files.
pub fn group_paths(paths: impl IntoIterator<Item = PathBuf>) -> ScanReport {
    let mut groups: BTreeMap<OsString, PhotoGroup> = BTreeMap::new();
    let mut unsupported_names = Vec::new();

    for path in paths {
        let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        let extension = extension.to_ascii_lowercase();
        let is_jpeg = matches!(extension.as_str(), "jpg" | "jpeg");
        if !is_jpeg && !RAW_EXTENSIONS.contains(&extension.as_str()) {
            continue;
        }
        let Some(stem) = path.file_stem() else {
            continue;
        };
        let Some(stem_text) = stem.to_str() else {
            unsupported_names.push(path);
            continue;
        };
        let group = groups
            .entry(stem.to_os_string())
            .or_insert_with(|| PhotoGroup::new(stem_text.to_owned()));
        if is_jpeg {
            group.jpegs.push(path);
        } else {
            group.raws.push(path);
        }
    }

    ScanReport {
        groups: groups.into_values().collect(),
        unsupported_names,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Decision;

    #[test]
    fn preserves_orphans_and_multiple_variants_without_default_rejection() {
        let report = group_paths(
            [
                "撮影.JPG",
                "撮影.NEF",
                "撮影.DNG",
                "orphan.arw",
                "solo.jpeg",
                "notes.txt",
            ]
            .map(PathBuf::from),
        );
        assert_eq!(report.groups.len(), 3);
        let paired = report.groups.iter().find(|g| g.stem == "撮影").unwrap();
        assert_eq!(paired.jpegs.len(), 1);
        assert_eq!(paired.raws.len(), 2);
        assert!(paired.is_ambiguous());
        assert!(report.groups.iter().any(|g| g.jpegs.is_empty()));
        assert!(
            report
                .groups
                .iter()
                .all(|g| g.decision == Decision::Unreviewed)
        );
    }

    #[test]
    fn does_not_silently_case_fold_stems() {
        let report = group_paths(["IMG.jpg", "img.nef"].map(PathBuf::from));
        assert_eq!(report.groups.len(), 2);
    }
}
