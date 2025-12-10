use anyhow::{Result, anyhow};
use std::{collections::HashSet, env, fs, path::Path};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub struct ExecutablesFinder {}

impl ExecutablesFinder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn find_executables_in_path(&self) -> Result<Vec<String>> {
        let path_env = env::var("PATH")?;
        let mut binaries = HashSet::new();

        for path in env::split_paths(&path_env) {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    if path.is_file() && self.is_executable(&path) {
                        if let Some(name) = path.file_name() {
                            binaries.insert(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(binaries.into_iter().collect())
    }

    fn is_executable(&self, path: &Path) -> bool {
        #[cfg(unix)]
        {
            if let Ok(metadata) = fs::metadata(path) {
                if !metadata.is_file() {
                    return false;
                }
                let permissions = metadata.permissions();
                return permissions.mode() & 0o111 != 0;
            }
            false
        }

        #[cfg(windows)]
        {
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                return ext == "exe" || ext == "bat" || ext == "cmd" || ext == "com";
            }
            false
        }
    }
}
