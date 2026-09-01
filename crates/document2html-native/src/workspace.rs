use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::fonts::substitution_registry;
use crate::{NativeError, NativeResult};

static WORKSPACE_COUNTER: AtomicU64 = AtomicU64::new(0);
const MAX_CREATE_ATTEMPTS: usize = 128;

pub(crate) struct TemporaryWorkspace {
    root: PathBuf,
}

impl TemporaryWorkspace {
    pub(crate) fn create() -> NativeResult<Self> {
        let temp = std::env::temp_dir();
        for _ in 0..MAX_CREATE_ATTEMPTS {
            let counter = WORKSPACE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let root = temp.join(format!("document2html-{}-{counter}", std::process::id()));
            match fs::create_dir(&root) {
                Ok(()) => return Self::initialize(root),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(NativeError::Io(error)),
            }
        }
        Err(NativeError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate an isolated temporary workspace",
        )))
    }

    fn initialize(root: PathBuf) -> NativeResult<Self> {
        set_owner_only_permissions(&root)?;
        let input = root.join("input");
        let office = root.join("office");
        let spreadsheet = root.join("spreadsheet");
        let poppler = root.join("poppler");
        let profile = root.join("profile");
        for directory in [
            &input,
            &office,
            &spreadsheet,
            &poppler,
            &profile,
            &root.join("home"),
            &root.join("tmp"),
        ] {
            if let Err(error) = fs::create_dir(directory) {
                if let Err(cleanup_error) = fs::remove_dir_all(&root) {
                    log::warn!(
                        "failed to clean partial native workspace {}: {cleanup_error}",
                        root.display()
                    );
                }
                return Err(NativeError::Io(error));
            }
        }
        Ok(Self { root })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Seeds the private LibreOffice profile with the east-Asian font
    /// replacement table. LibreOffice merges this file on first launch, so it
    /// must be written before any stage runs.
    pub(crate) fn seed_font_substitution(&self, substitute: &str) -> NativeResult<()> {
        let user = self.root.join("profile").join("user");
        fs::create_dir_all(&user)?;
        fs::write(
            user.join("registrymodifications.xcu"),
            substitution_registry(substitute),
        )?;
        Ok(())
    }
}

impl Drop for TemporaryWorkspace {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            log::warn!(
                "failed to clean native workspace {}: {error}",
                self.root.display()
            );
        }
    }
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> NativeResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> NativeResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::TemporaryWorkspace;

    #[test]
    fn creates_isolated_layout_and_cleans_it_on_drop() {
        // Given
        let workspace = TemporaryWorkspace::create().expect("create workspace");
        let root = workspace.root().to_owned();

        // When
        assert!(workspace.root().join("input").is_dir());
        assert!(workspace.root().join("office").is_dir());
        assert!(workspace.root().join("spreadsheet").is_dir());
        assert!(workspace.root().join("poppler").is_dir());
        assert!(workspace.root().join("profile").is_dir());
        drop(workspace);

        // Then
        assert!(!root.exists());
    }

    #[test]
    fn concurrent_workspaces_never_share_a_root() {
        // Given
        let first = TemporaryWorkspace::create().expect("create first workspace");
        let second = TemporaryWorkspace::create().expect("create second workspace");

        // When
        let roots_are_distinct = first.root() != second.root();

        // Then
        assert!(roots_are_distinct);
    }
}
