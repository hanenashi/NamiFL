//! Android transport boundary. No processes are spawned in the skeleton.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub const CAMERA_DIRECTORY: &str = "/storage/emulated/0/DCIM/Camera";

/// An explicit target is required; device discovery/authorization is future work.
#[derive(Clone, Debug)]
pub struct AdbTarget {
    pub executable: PathBuf,
    serial: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidSerial;

impl AdbTarget {
    pub fn new(executable: PathBuf, serial: String) -> Result<Self, InvalidSerial> {
        if serial.is_empty() || serial.starts_with('-') || serial.chars().any(char::is_whitespace) {
            return Err(InvalidSerial);
        }
        Ok(Self { executable, serial })
    }

    /// Arguments for a future process runner. Pass directly, never via a shell.
    /// The caller must validate the remote path against its destination policy.
    pub fn push_args(&self, source: &Path, remote_path: &str) -> Vec<OsString> {
        vec![
            "-s".into(),
            self.serial.clone().into(),
            "push".into(),
            source.as_os_str().to_owned(),
            remote_path.into(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_the_device_and_preserves_paths_as_single_arguments() {
        let target = AdbTarget::new("adb.exe".into(), "device-123".into()).unwrap();
        let source = Path::new("exports/my photo.jpg");
        let args = target.push_args(source, "/sdcard/DCIM/Camera/my photo.jpg");
        assert_eq!(args[0], "-s");
        assert_eq!(args[1], "device-123");
        assert_eq!(args.len(), 5);
        assert_eq!(args[3], source.as_os_str());
        assert!(AdbTarget::new("adb".into(), String::new()).is_err());
    }
}
