use std::collections::BTreeSet;
use std::path::PathBuf;

/// Source paths remain unchanged; destination naming is a separate concern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportItem {
    pub source: PathBuf,
    pub destination_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    NotJpeg(PathBuf),
    UnsupportedName(PathBuf),
    InvalidSuffix,
    DuplicateDestination(String),
}

/// Draft a JPEG-only plan from an explicitly chosen DxO export selection.
/// No filesystem access, remote conflict checks, or source renaming occurs.
pub fn plan_exports(
    sources: impl IntoIterator<Item = PathBuf>,
    suffix: &str,
) -> Result<Vec<ExportItem>, PlanError> {
    if suffix
        .chars()
        .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err(PlanError::InvalidSuffix);
    }
    let mut names = BTreeSet::new();
    let mut items = Vec::new();
    for source in sources {
        let extension = source.extension().and_then(|v| v.to_str()).unwrap_or("");
        if !extension.eq_ignore_ascii_case("jpg") && !extension.eq_ignore_ascii_case("jpeg") {
            return Err(PlanError::NotJpeg(source));
        }
        let Some(stem) = source.file_stem().and_then(|v| v.to_str()) else {
            return Err(PlanError::UnsupportedName(source));
        };
        let destination_name = format!("{stem}{suffix}.{extension}");
        // Conservative draft collision check. Remote checks belong to the adapter.
        if !names.insert(destination_name.to_lowercase()) {
            return Err(PlanError::DuplicateDestination(destination_name));
        }
        items.push(ExportItem {
            source,
            destination_name,
        });
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_changes_only_the_destination() {
        let source = PathBuf::from("DxO/撮影.jpg");
        let plan = plan_exports([source.clone()], "_ok").unwrap();
        assert_eq!(plan[0].source, source);
        assert_eq!(plan[0].destination_name, "撮影_ok.jpg");
    }

    #[test]
    fn rejects_raws_and_destination_collisions() {
        assert!(matches!(
            plan_exports([PathBuf::from("photo.nef")], ""),
            Err(PlanError::NotJpeg(_))
        ));
        assert!(matches!(
            plan_exports(["a/photo.jpg", "b/PHOTO.JPG"].map(PathBuf::from), ""),
            Err(PlanError::DuplicateDestination(_))
        ));
        assert_eq!(plan_exports([], "../"), Err(PlanError::InvalidSuffix));
    }
}
