//! Explicit cleanup policy: leave keeper JPEGs; unreviewed groups are excluded.

use crate::domain::{Decision, PhotoGroup};
use crate::review::FileStamp;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct DeletePlan {
    pub files: Vec<FileStamp>,
    pub jpeg_count: usize,
    pub raw_count: usize,
    pub keeper_jpegs: usize,
    pub unreviewed_groups: usize,
}

pub fn plan_deletion(
    groups: &[PhotoGroup],
    stamps: &BTreeMap<String, Vec<FileStamp>>,
) -> Result<DeletePlan, String> {
    let mut plan = DeletePlan::default();
    for group in groups {
        if group.decision == Decision::Unreviewed {
            plan.unreviewed_groups += 1;
            continue;
        }
        if group.decision == Decision::Keep {
            plan.keeper_jpegs += group.jpegs.len();
        }
        for path in group.raws.iter().chain(
            group
                .jpegs
                .iter()
                .filter(|_| group.decision == Decision::Reject),
        ) {
            let stamp = stamps
                .get(&group.stem)
                .and_then(|items| items.iter().find(|stamp| &stamp.path == path))
                .ok_or_else(|| {
                    format!(
                        "Source metadata missing for {}. Reopen the folder.",
                        path.display()
                    )
                })?;
            if group.jpegs.contains(path) {
                plan.jpeg_count += 1;
            } else {
                plan.raw_count += 1;
            }
            plan.files.push(stamp.clone());
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leaves_keeper_jpegs_and_all_unreviewed_files() {
        let groups: Vec<_> = [Decision::Keep, Decision::Reject, Decision::Unreviewed]
            .into_iter()
            .enumerate()
            .map(|(i, decision)| PhotoGroup {
                stem: i.to_string(),
                jpegs: vec![format!("{i}.jpg").into()],
                raws: vec![format!("{i}.dng").into()],
                decision,
            })
            .collect();
        let stamps = groups
            .iter()
            .map(|g| {
                (
                    g.stem.clone(),
                    g.jpegs
                        .iter()
                        .chain(&g.raws)
                        .map(|path| FileStamp {
                            path: path.clone(),
                            bytes: 1,
                            modified_ns: 1,
                        })
                        .collect(),
                )
            })
            .collect();
        let plan = plan_deletion(&groups, &stamps).unwrap();
        assert_eq!(
            (
                plan.jpeg_count,
                plan.raw_count,
                plan.keeper_jpegs,
                plan.unreviewed_groups
            ),
            (1, 2, 1, 1)
        );
        assert_eq!(
            plan.files
                .iter()
                .map(|f| f.path.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            ["0.dng", "1.dng", "1.jpg"]
        );
    }
}
