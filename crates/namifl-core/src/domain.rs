use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Review state never implies a filesystem operation.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Decision {
    #[default]
    Unreviewed,
    Keep,
    Reject,
}

/// A directory-local, exact-stem group. All variants are retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhotoGroup {
    pub stem: String,
    pub jpegs: Vec<PathBuf>,
    pub raws: Vec<PathBuf>,
    pub decision: Decision,
}

impl PhotoGroup {
    pub fn new(stem: String) -> Self {
        Self {
            stem,
            jpegs: Vec::new(),
            raws: Vec::new(),
            decision: Decision::Unreviewed,
        }
    }

    /// Multiple JPEGs or RAWs require explicit user resolution before bulk actions.
    pub fn is_ambiguous(&self) -> bool {
        self.jpegs.len() > 1 || self.raws.len() > 1
    }
}
