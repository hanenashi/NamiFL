//! Background folder/session I/O and explicitly confirmed cleanup execution.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;
use namifl_core::deletion::DeletePlan;
use namifl_core::discovery::scan_folder;
use namifl_core::domain::PhotoGroup;
use namifl_core::review::{FileStamp, ReviewSession, SESSION_VERSION, fingerprint};
use sha2::{Digest, Sha256};

pub struct LoadedFolder {
    pub groups: Vec<PhotoGroup>,
    pub session: ReviewSession,
    pub stamps: BTreeMap<String, Vec<FileStamp>>,
    pub unsupported_names: usize,
}

pub enum IoRequest {
    Preferences(crate::settings::Settings),
    Delete {
        folder: PathBuf,
        plan: DeletePlan,
        session: ReviewSession,
        generation: u64,
    },
    Open {
        folder: Option<PathBuf>,
        generation: u64,
    },
    Save {
        session: ReviewSession,
        revision: u64,
        generation: u64,
    },
}

pub enum IoResult {
    PreferencesSaved(Result<(), String>),
    Deleted {
        result: Result<(LoadedFolder, DeleteReport), String>,
        generation: u64,
    },
    Opened {
        result: Result<Option<LoadedFolder>, String>,
        generation: u64,
    },
    Saved {
        result: Result<(), String>,
        revision: u64,
        generation: u64,
    },
}

pub struct SessionWorker {
    pub tx: Sender<IoRequest>,
    pub rx: Receiver<IoResult>,
}

impl SessionWorker {
    pub fn start(ctx: egui::Context, root: PathBuf) -> Self {
        let (tx, requests) = mpsc::channel();
        let (results, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for request in requests {
                let result = match request {
                    IoRequest::Preferences(settings) => IoResult::PreferencesSaved(
                        crate::settings::save(&root, &settings).map_err(|e| e.to_string()),
                    ),
                    IoRequest::Delete {
                        folder,
                        plan,
                        session,
                        generation,
                    } => {
                        let result = cleanup_and_reload(&root, &folder, &plan, &session)
                            .map_err(|error| error.to_string());
                        IoResult::Deleted { result, generation }
                    }
                    IoRequest::Open { folder, generation } => {
                        let folder = folder.or_else(|| {
                            rfd::FileDialog::new()
                                .set_title("Open camera JPEG + RAW folder")
                                .pick_folder()
                        });
                        let result = folder
                            .map(|folder| load_folder(&root, &folder))
                            .transpose()
                            .map_err(|e| e.to_string());
                        IoResult::Opened { result, generation }
                    }
                    IoRequest::Save {
                        session,
                        revision,
                        generation,
                    } => {
                        let result = save_session(&root, &session).map_err(|e| e.to_string());
                        IoResult::Saved {
                            result,
                            revision,
                            generation,
                        }
                    }
                };
                if results.send(result).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self { tx, rx }
    }
}

#[derive(Debug, Default)]
pub struct DeleteReport {
    pub removed: usize,
    pub failed: Vec<(PathBuf, String)>,
}

fn cleanup_and_reload(
    root: &Path,
    folder: &Path,
    plan: &DeletePlan,
    previous: &ReviewSession,
) -> io::Result<(LoadedFolder, DeleteReport)> {
    let report = execute_deletion(folder, plan)?;
    let mut loaded = load_folder(root, folder)?;
    for group in &mut loaded.groups {
        let files = &loaded.stamps[&group.stem];
        if let Some(old) = previous.records.get(&group.stem)
            && files.iter().all(|file| old.files.contains(file))
        {
            group.decision = old.decision;
        }
        loaded.session.records.insert(
            group.stem.clone(),
            namifl_core::review::ReviewRecord {
                files: files.clone(),
                decision: group.decision,
            },
        );
    }
    save_session(root, &loaded.session)?;
    Ok((loaded, report))
}

fn execute_deletion(folder: &Path, plan: &DeletePlan) -> io::Result<DeleteReport> {
    let folder = folder.canonicalize()?;
    let mut report = DeleteReport::default();
    for expected in &plan.files {
        let result = (|| -> io::Result<()> {
            let actual_path = expected.path.canonicalize()?;
            if actual_path.parent() != Some(folder.as_path()) {
                return Err(io::Error::other(
                    "File is no longer directly inside the reviewed folder",
                ));
            }
            let metadata = fs::symlink_metadata(&expected.path)?;
            let modified_ns = metadata
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_nanos();
            if !metadata.file_type().is_file()
                || metadata.len() != expected.bytes
                || modified_ns != expected.modified_ns
            {
                return Err(io::Error::other(
                    "Source changed since review; reopen the folder",
                ));
            }
            fs::remove_file(&expected.path)
        })();
        match result {
            Ok(()) => report.removed += 1,
            Err(error) => report
                .failed
                .push((expected.path.clone(), error.to_string())),
        }
    }
    Ok(report)
}

fn session_path(root: &Path, folder: &Path) -> PathBuf {
    let digest = Sha256::digest(folder.as_os_str().as_encoded_bytes());
    root.join(format!("{digest:x}.json"))
}

fn load_folder(root: &Path, folder: &Path) -> io::Result<LoadedFolder> {
    let folder = folder.canonicalize()?;
    let report = scan_folder(&folder)?;
    let file = session_path(root, &folder);
    let session = match File::open(file) {
        Ok(file) => {
            let session: ReviewSession =
                serde_json::from_reader(BufReader::new(file)).map_err(|e| {
                    io::Error::other(format!(
                        "Saved review could not be read: {e}. It has not been overwritten."
                    ))
                })?;
            if session.version != SESSION_VERSION || session.folder != folder {
                return Err(io::Error::other(
                    "Saved review version or folder does not match. It has not been overwritten.",
                ));
            }
            session
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => ReviewSession::new(folder),
        Err(error) => return Err(error),
    };
    let mut groups = report.groups;
    let mut stamps = BTreeMap::new();
    for group in &mut groups {
        let files = fingerprint(group)?;
        group.decision = session.restored_decision(&group.stem, &files);
        stamps.insert(group.stem.clone(), files);
    }
    Ok(LoadedFolder {
        groups,
        session,
        stamps,
        unsupported_names: report.unsupported_names.len(),
    })
}

fn save_session(root: &Path, session: &ReviewSession) -> io::Result<()> {
    fs::create_dir_all(root)?;
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    serde_json::to_writer_pretty(file.as_file_mut(), session).map_err(io::Error::other)?;
    file.flush()?;
    file.as_file().sync_all()?;
    file.persist(session_path(root, &session.folder))
        .map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use namifl_core::domain::Decision;
    use namifl_core::review::ReviewRecord;

    #[test]
    fn cleanup_preserves_keeper_jpeg_and_skips_a_changed_source() {
        let photos = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        for name in [
            "keep.jpg",
            "keep.dng",
            "reject.jpg",
            "reject.dng",
            "unreviewed.jpg",
        ] {
            fs::write(photos.path().join(name), b"fixture").unwrap();
        }
        let mut loaded = load_folder(state.path(), photos.path()).unwrap();
        for group in &mut loaded.groups {
            group.decision = match group.stem.as_str() {
                "keep" => Decision::Keep,
                "reject" => Decision::Reject,
                _ => Decision::Unreviewed,
            };
            loaded.session.records.insert(
                group.stem.clone(),
                ReviewRecord {
                    files: loaded.stamps[&group.stem].clone(),
                    decision: group.decision,
                },
            );
        }
        let plan = namifl_core::deletion::plan_deletion(&loaded.groups, &loaded.stamps).unwrap();
        fs::write(
            photos.path().join("reject.dng"),
            b"changed source after plan",
        )
        .unwrap();
        let (after, report) =
            cleanup_and_reload(state.path(), photos.path(), &plan, &loaded.session).unwrap();
        assert_eq!(report.removed, 2);
        assert_eq!(report.failed.len(), 1);
        assert!(photos.path().join("keep.jpg").exists());
        assert!(!photos.path().join("keep.dng").exists());
        assert!(!photos.path().join("reject.jpg").exists());
        assert!(photos.path().join("reject.dng").exists());
        assert!(photos.path().join("unreviewed.jpg").exists());
        assert_eq!(
            after
                .groups
                .iter()
                .find(|g| g.stem == "keep")
                .unwrap()
                .decision,
            Decision::Keep
        );
    }

    #[test]
    fn cleanup_refuses_targets_outside_the_reviewed_folder() {
        let photos = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let path = outside.path().join("outside.jpg");
        fs::write(&path, b"leave this file").unwrap();
        let group = PhotoGroup {
            stem: "outside".into(),
            jpegs: vec![path.clone()],
            raws: Vec::new(),
            decision: Decision::Reject,
        };
        let plan = DeletePlan {
            files: fingerprint(&group).unwrap(),
            ..Default::default()
        };
        let report = execute_deletion(photos.path(), &plan).unwrap();
        assert_eq!(report.removed, 0);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(fs::read(path).unwrap(), b"leave this file");
    }

    #[test]
    fn session_round_trip_preserves_sources_and_resets_a_changed_group() {
        let photos = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let jpeg = photos.path().join("撮影.jpg");
        fs::write(&jpeg, b"synthetic jpeg bytes").unwrap();
        fs::write(photos.path().join("撮影.nef"), b"raw untouched").unwrap();
        let mut loaded = load_folder(state.path(), photos.path()).unwrap();
        loaded.session.records.insert(
            "撮影".into(),
            ReviewRecord {
                files: loaded.stamps["撮影"].clone(),
                decision: Decision::Keep,
            },
        );
        save_session(state.path(), &loaded.session).unwrap();
        // Exercise replacement of an existing session on Windows as well.
        save_session(state.path(), &loaded.session).unwrap();
        assert_eq!(
            load_folder(state.path(), photos.path()).unwrap().groups[0].decision,
            Decision::Keep
        );
        assert_eq!(fs::read(&jpeg).unwrap(), b"synthetic jpeg bytes");
        assert_eq!(
            fs::read(photos.path().join("撮影.nef")).unwrap(),
            b"raw untouched"
        );
        fs::write(&jpeg, b"changed and longer synthetic jpeg").unwrap();
        assert_eq!(
            load_folder(state.path(), photos.path()).unwrap().groups[0].decision,
            Decision::Unreviewed
        );
    }

    #[test]
    fn corrupt_or_future_sessions_are_preserved() {
        let photos = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let folder = photos.path().canonicalize().unwrap();
        let path = session_path(state.path(), &folder);
        fs::write(&path, b"broken").unwrap();
        assert!(load_folder(state.path(), &folder).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"broken");
        let mut future = ReviewSession::new(folder.clone());
        future.version += 1;
        fs::write(&path, serde_json::to_vec(&future).unwrap()).unwrap();
        assert!(load_folder(state.path(), &folder).is_err());
    }
}
