//! Private Unix output bundles; descriptor-relative files, exclusive creation, no replacement.
use super::{Result, invalid};
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

pub(super) struct Bundle {
    path: PathBuf,
    directory: File,
}

fn no_symlinks(path: &Path) -> Result<()> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut current = PathBuf::new();
    for c in path.components() {
        if matches!(c, Component::ParentDir) {
            return Err(invalid("Directory traversal is refused"));
        }
        current.push(c);
        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err(invalid("Symlink path component refused"));
        }
    }
    Ok(())
}

impl Bundle {
    pub(super) fn create(path: &Path) -> Result<Self> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        no_symlinks(parent)?;
        if path.file_name().is_none()
            || path.components().any(|c| matches!(c, Component::ParentDir))
        {
            return Err(invalid("Output must be a new ordinary directory"));
        }
        let path = fs::canonicalize(parent)?.join(
            path.file_name()
                .ok_or_else(|| invalid("Missing directory name"))?,
        );
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|_| {
                invalid(
                    "Cannot create new acquisition directory; existing paths are never overwritten",
                )
            })?;
        Self::open(&path)
    }

    pub(super) fn open(path: &Path) -> Result<Self> {
        no_symlinks(path)?;
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        let bundle = Self {
            path: path.to_owned(),
            directory,
        };
        bundle.finish()?;
        Ok(bundle)
    }

    fn open_child(&self, name: &str, write: bool) -> Result<File> {
        self.finish()?;
        if name.is_empty() || name.contains(['/', '\\', '\0']) || name == "." || name == ".." {
            return Err(invalid("Unsafe acquisition child path"));
        }
        let name = CString::new(name).map_err(|_| invalid("Unsafe acquisition path"))?;
        let flags = libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK
            | if write {
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL
            } else {
                libc::O_RDONLY
            };
        let fd = unsafe { libc::openat(self.directory.as_raw_fd(), name.as_ptr(), flags, 0o600) };
        if fd < 0 {
            return Err(invalid(
                "Acquisition file open/create refused; symlinks and overwrites are forbidden",
            ));
        }
        let file = unsafe { File::from_raw_fd(fd) };
        let m = file.metadata()?;
        if !m.is_file() || m.nlink() != 1 {
            return Err(invalid(
                "Acquisition files must be ordinary files with one link",
            ));
        }
        Ok(file)
    }

    pub(super) fn create_file(&self, name: &str) -> Result<File> {
        self.open_child(name, true)
    }
    pub(super) fn read_file(&self, name: &str) -> Result<File> {
        self.open_child(name, false)
    }

    pub(super) fn publish(&self, partial: &str, final_name: &str) -> Result<()> {
        self.finish()?;
        let from = CString::new(partial).map_err(|_| invalid("Unsafe publication path"))?;
        let to = CString::new(final_name).map_err(|_| invalid("Unsafe publication path"))?;
        // linkat refuses an existing destination; rename would silently replace it.
        if unsafe {
            libc::linkat(
                self.directory.as_raw_fd(),
                from.as_ptr(),
                self.directory.as_raw_fd(),
                to.as_ptr(),
                0,
            )
        } != 0
        {
            return Err(invalid(
                "Payload publication refused; no existing destination is replaced",
            ));
        }
        if unsafe { libc::unlinkat(self.directory.as_raw_fd(), from.as_ptr(), 0) } != 0 {
            return Err(invalid("Payload publication incomplete"));
        }
        self.read_file(final_name)?
            .set_permissions(fs::Permissions::from_mode(0o400))?;
        self.directory.sync_all()?;
        self.finish()
    }

    pub(super) fn exact_files(&self, names: &[&str]) -> Result<()> {
        self.finish()?;
        let actual = fs::read_dir(&self.path)?
            .map(|entry| entry.map(|e| e.file_name()))
            .collect::<std::io::Result<Vec<_>>>()?;
        if actual.len() != names.len()
            || actual
                .iter()
                .any(|n| n.to_str().is_none_or(|n| !names.contains(&n)))
        {
            return Err(invalid("Unexpected/missing acquisition files"));
        }
        Ok(())
    }

    pub(super) fn finish(&self) -> Result<()> {
        let observed = fs::symlink_metadata(&self.path)?;
        let open = self.directory.metadata()?;
        if !observed.is_dir()
            || observed.file_type().is_symlink()
            || observed.dev() != open.dev()
            || observed.ino() != open.ino()
        {
            return Err(invalid("Acquisition directory identity changed"));
        }
        Ok(())
    }
}
