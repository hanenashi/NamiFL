//! Durable review records. Identity is directory-local and path-based;
//! modified files are deliberately treated as new review inputs.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::domain::{Decision, PhotoGroup};

pub const SESSION_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileStamp {
    pub path: PathBuf,
    pub bytes: u64,
    pub modified_ns: u128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewRecord {
    pub files: Vec<FileStamp>,
    pub decision: Decision,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReviewSession {
    pub version: u32,
    pub folder: PathBuf,
    pub records: BTreeMap<String, ReviewRecord>,
}

impl ReviewSession {
    pub fn new(folder: PathBuf) -> Self {
        Self {
            version: SESSION_VERSION,
            folder,
            records: BTreeMap::new(),
        }
    }

    pub fn restored_decision(&self, stem: &str, files: &[FileStamp]) -> Decision {
        self.records
            .get(stem)
            .filter(|record| record.files == files)
            .map_or(Decision::Unreviewed, |record| record.decision)
    }
}

/// Metadata fingerprint, not a content hash. Same-size edits with an unchanged
/// modification time cannot be detected and must not be used for transfer verification.
pub fn fingerprint(group: &PhotoGroup) -> io::Result<Vec<FileStamp>> {
    let mut stamps = Vec::new();
    for path in group.jpegs.iter().chain(&group.raws) {
        let metadata = fs::metadata(path)?;
        let modified_ns = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        stamps.push(FileStamp {
            path: path.clone(),
            bytes: metadata.len(),
            modified_ns,
        });
    }
    stamps.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(stamps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_only_when_every_source_matches() {
        let original = vec![FileStamp {
            path: "a.jpg".into(),
            bytes: 10,
            modified_ns: 100,
        }];
        let mut session = ReviewSession::new("shoot".into());
        session.records.insert(
            "a".into(),
            ReviewRecord {
                files: original.clone(),
                decision: Decision::Keep,
            },
        );
        assert_eq!(session.restored_decision("a", &original), Decision::Keep);
        let mut modified = original.clone();
        modified[0].modified_ns += 1;
        assert_eq!(
            session.restored_decision("a", &modified),
            Decision::Unreviewed
        );
        let mut added = original.clone();
        added.push(FileStamp {
            path: "a.nef".into(),
            bytes: 90,
            modified_ns: 100,
        });
        assert_eq!(session.restored_decision("a", &added), Decision::Unreviewed);
        assert_eq!(
            session.restored_decision("missing", &original),
            Decision::Unreviewed
        );
    }
}
