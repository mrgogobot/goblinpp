//! Unix descriptor-relative traversal. Names from target trees are never executed.
use super::{Entry, Fingerprint, Kind, Limits, Observation, POLICY, Snapshot, now, safe_relative};
use crate::error::Result;
use sha2::{Digest, Sha256};
use std::ffi::{CStr, CString, OsStr};
use std::fs::{File, Metadata};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};
use std::time::{Duration, Instant};

fn fingerprint(m: &Metadata) -> Fingerprint {
    Fingerprint {
        bytes: m.len(),
        device: m.dev(),
        inode: m.ino(),
        mode: m.mode(),
        modified: format!("{}:{}", m.mtime(), m.mtime_nsec()),
        changed: format!("{}:{}", m.ctime(), m.ctime_nsec()),
    }
}

/// Never follows the final component. Directory handles keep recursion anchored.
fn open_at(parent: &File, name: &OsStr, directory: bool) -> io::Result<File> {
    let name = CString::new(name.as_bytes()).map_err(|_| io::Error::other("NUL path"))?;
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | libc::O_NOCTTY
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: valid borrowed directory fd and NUL-terminated name; returned fd owned below.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: successful openat returned an exclusively owned fd.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

fn root_handle(root: &Path) -> io::Result<File> {
    let mut file = File::open("/")?;
    for component in root.components() {
        if let Component::Normal(name) = component {
            file = open_at(&file, name, true)?;
        }
    }
    Ok(file)
}

struct Directory(*mut libc::DIR);
impl Drop for Directory {
    fn drop(&mut self) {
        // SAFETY: fdopendir owns this valid DIR until this single closedir call.
        unsafe {
            libc::closedir(self.0);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn errno() -> *mut libc::c_int {
    // SAFETY: libc supplies a valid thread-local errno pointer.
    unsafe { libc::__errno_location() }
}
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly"
))]
fn errno() -> *mut libc::c_int {
    // SAFETY: libc supplies a valid thread-local errno pointer.
    unsafe { libc::__error() }
}
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly"
)))]
compile_error!(
    "Orbit's Unix scanner currently supports Linux/macOS and their listed libc relatives only"
);

fn names(file: &File, remaining: usize, deadline: Instant) -> io::Result<Vec<Vec<u8>>> {
    // SAFETY: duplicate a valid borrowed fd; fdopendir takes ownership only on success.
    let fd = unsafe { libc::dup(file.as_raw_fd()) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the duplicate is a live directory descriptor.
    let pointer = unsafe { libc::fdopendir(fd) };
    if pointer.is_null() {
        let error = io::Error::last_os_error();
        // SAFETY: fdopendir failed, so ownership remained with us.
        unsafe {
            libc::close(fd);
        }
        return Err(error);
    }
    let directory = Directory(pointer);
    let mut entries = Vec::new();
    loop {
        if Instant::now() >= deadline {
            return Err(io::Error::other("wall-clock budget exhausted"));
        }
        // SAFETY: errno pointer valid, directory live; copy readdir storage before next call.
        let entry = unsafe {
            *errno() = 0;
            libc::readdir(directory.0)
        };
        if entry.is_null() {
            // SAFETY: errno is thread-local and read immediately after readdir.
            if unsafe { *errno() } != 0 {
                return Err(io::Error::last_os_error());
            }
            break;
        }
        // SAFETY: POSIX readdir supplies a terminated d_name for the current entry.
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        if entries.len() >= remaining {
            return Err(io::Error::other(
                "entry budget exhausted; directory inventory incomplete",
            ));
        }
        entries.push(name.to_vec());
    }
    entries.sort();
    Ok(entries)
}

fn kind_at(parent: &File, name: &OsStr) -> io::Result<libc::mode_t> {
    let name = CString::new(name.as_bytes()).map_err(|_| io::Error::other("NUL path"))?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: output buffer valid, parent/name live; successful call initializes stat.
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fstatat succeeded.
    Ok(unsafe { stat.assume_init() }.st_mode & libc::S_IFMT)
}

struct Scan<'a> {
    limits: &'a Limits,
    deadline: Instant,
    bytes: u64,
    entries: Vec<Entry>,
    issues: Vec<String>,
}

impl Scan<'_> {
    fn issue(&mut self, message: String) {
        // Keep diagnostics bounded even with many unreadable/unsafe entries.
        if self.issues.len() < 64 {
            self.issues.push(message);
        }
    }

    fn file(&mut self, directory: &File, name: &OsStr, path: &str) -> Entry {
        self.file_observe(directory, name, path, || {})
    }

    fn file_observe(
        &mut self,
        directory: &File,
        name: &OsStr,
        path: &str,
        after_open: impl FnOnce(),
    ) -> Entry {
        let mut entry = Entry {
            path: path.into(),
            kind: Kind::File,
            observation: Observation::Unreadable,
            bytes_read: 0,
            sha256: None,
            before: None,
            after: None,
        };
        let Ok(mut file) = open_at(directory, name, false) else {
            return entry;
        };
        let Ok(before) = file.metadata() else {
            return entry;
        };
        if !before.is_file() {
            entry.kind = Kind::Other;
            entry.observation = Observation::Unsupported;
            return entry;
        }
        entry.before = Some(fingerprint(&before));
        if before.len() > self.limits.max_file_bytes
            || before.len() > self.limits.max_total_bytes - self.bytes
        {
            entry.observation = Observation::BudgetExceeded;
            return entry;
        }
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        after_open();
        // Hash exactly the observed size, then compare full metadata and directory binding.
        while entry.bytes_read < before.len() {
            if Instant::now() >= self.deadline {
                entry.observation = Observation::BudgetExceeded;
                return entry;
            }
            let remaining = (before.len() - entry.bytes_read).min(buffer.len() as u64) as usize;
            match file.read(&mut buffer[..remaining]) {
                Ok(0) => {
                    entry.observation = Observation::UnstableDuringRead;
                    return entry;
                }
                Ok(count) => {
                    entry.bytes_read += count as u64;
                    self.bytes += count as u64;
                    digest.update(&buffer[..count]);
                }
                Err(_) => return entry,
            }
        }
        let Ok(after) = file.metadata() else {
            return entry;
        };
        entry.after = Some(fingerprint(&after));
        // Reopen through the anchored directory to detect pathname replacement.
        let binding = open_at(directory, name, false)
            .and_then(|f| f.metadata())
            .ok();
        if entry.before != entry.after || binding.as_ref().map(fingerprint) != entry.after {
            entry.observation = Observation::UnstableDuringRead;
        } else {
            entry.observation = Observation::Hashed;
            entry.sha256 = Some(format!("{:x}", digest.finalize()));
        }
        entry
    }

    fn walk(&mut self, directory: &File, prefix: &str, depth: usize) {
        let before = directory.metadata().ok().map(|m| fingerprint(&m));
        let names = match names(
            directory,
            self.limits.max_entries - self.entries.len(),
            self.deadline,
        ) {
            Ok(names) => names,
            Err(_) => {
                self.issue(format!(
                    "Directory unreadable or entry/time budget exhausted at {}",
                    if prefix.is_empty() { "<root>" } else { prefix }
                ));
                return;
            }
        };
        for name in names {
            if self.entries.len() >= self.limits.max_entries || Instant::now() >= self.deadline {
                self.issue(
                    "Entry/time budget exhausted; remaining names were not inspected".into(),
                );
                break;
            }
            let Ok(text) = std::str::from_utf8(&name) else {
                self.issue("Non-UTF-8 filename encountered; coverage incomplete".into());
                continue;
            };
            let path = if prefix.is_empty() {
                text.to_owned()
            } else {
                format!("{prefix}/{text}")
            };
            if !safe_relative(&path) {
                self.issue("Unsafe or oversized filename encountered; coverage incomplete".into());
                continue;
            }
            let name = OsStr::from_bytes(&name);
            let kind = kind_at(directory, name);
            match kind {
                Ok(libc::S_IFREG) => {
                    let entry = self.file(directory, name, &path);
                    self.entries.push(entry);
                }
                Ok(libc::S_IFDIR) => {
                    let index = self.entries.len();
                    let mut entry = Entry {
                        path: path.clone(),
                        kind: Kind::Directory,
                        observation: Observation::Unreadable,
                        bytes_read: 0,
                        sha256: None,
                        before: None,
                        after: None,
                    };
                    let child = open_at(directory, name, true).ok();
                    if let Some(child) = &child {
                        entry.before = child.metadata().ok().map(|m| fingerprint(&m));
                    }
                    self.entries.push(entry);
                    if depth >= self.limits.max_depth {
                        self.entries[index].observation = Observation::BudgetExceeded;
                    } else if let Some(child) = child {
                        self.walk(&child, &path, depth + 1);
                        let after = child.metadata().ok().map(|m| fingerprint(&m));
                        let binding = open_at(directory, name, true)
                            .and_then(|f| f.metadata())
                            .ok()
                            .map(|m| fingerprint(&m));
                        let entry = &mut self.entries[index];
                        entry.after = after;
                        entry.observation = if entry.before.is_some()
                            && entry.before == entry.after
                            && binding == entry.after
                        {
                            Observation::Directory
                        } else {
                            Observation::UnstableDuringRead
                        };
                    }
                }
                Ok(_) => self.entries.push(Entry {
                    path,
                    kind: Kind::Other,
                    observation: Observation::Unsupported,
                    bytes_read: 0,
                    sha256: None,
                    before: None,
                    after: None,
                }),
                Err(_) => self.entries.push(Entry {
                    path,
                    kind: Kind::Other,
                    observation: Observation::Unreadable,
                    bytes_read: 0,
                    sha256: None,
                    before: None,
                    after: None,
                }),
            }
        }
        if before.is_none() || directory.metadata().ok().map(|m| fingerprint(&m)) != before {
            self.issue(format!(
                "Directory changed during observation at {}",
                if prefix.is_empty() { "<root>" } else { prefix }
            ));
        }
    }
}

pub(super) fn scan(root: &Path, limits: &Limits) -> Result<Snapshot> {
    let started_utc = now();
    let mut state = Scan {
        limits,
        deadline: Instant::now() + Duration::from_secs(limits.max_seconds),
        bytes: 0,
        entries: vec![],
        issues: vec![],
    };
    if let Ok(directory) = root_handle(root) {
        state.walk(&directory, "", 0);
        let original = directory.metadata().ok().map(|m| fingerprint(&m));
        let binding = root_handle(root)
            .and_then(|f| f.metadata())
            .ok()
            .map(|m| fingerprint(&m));
        if original.is_none() || original != binding {
            state.issue("Root pathname changed during observation".into());
        }
    } else {
        state.issue("Selected root unavailable for anchored directory observation".into());
    }
    if Instant::now() >= state.deadline {
        state.issue("Wall-clock budget exhausted before observation completed".into());
    }
    state.entries.sort_by(|a, b| a.path.cmp(&b.path));
    let complete = state.issues.is_empty()
        && state
            .entries
            .iter()
            .all(|e| matches!(e.observation, Observation::Hashed | Observation::Directory));
    Ok(Snapshot {
        schema: "goblin.orbit-snapshot.v1".into(),
        policy: POLICY.into(),
        tool_version: crate::VERSION.into(),
        root: root.to_string_lossy().into_owned(),
        started_utc,
        finished_utc: now(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        limits: limits.clone(),
        bytes_read: state.bytes,
        complete,
        issues: state.issues,
        entries: state.entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadline_reports_incomplete_not_clean() {
        let temp = tempfile::tempdir().unwrap();
        let file = root_handle(&temp.path().canonicalize().unwrap()).unwrap();
        let limits = Limits::default();
        let mut state = Scan {
            limits: &limits,
            deadline: Instant::now(),
            bytes: 0,
            entries: vec![],
            issues: vec![],
        };
        state.walk(&file, "", 0);
        assert!(!state.issues.is_empty());
    }

    #[test]
    fn changed_file_and_path_replacement_never_get_authoritative_hashes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::write(root.join("a"), b"original").unwrap();
        let file = root_handle(&root).unwrap();
        let limits = Limits::default();
        let mut state = Scan {
            limits: &limits,
            deadline: Instant::now() + Duration::from_secs(10),
            bytes: 0,
            entries: vec![],
            issues: vec![],
        };
        let entry = state.file_observe(&file, OsStr::new("a"), "a", || {
            std::fs::write(root.join("a"), b"changed contents").unwrap();
        });
        assert_eq!(entry.observation, Observation::UnstableDuringRead);
        assert!(entry.sha256.is_none());
        let entry = state.file_observe(&file, OsStr::new("a"), "a", || {
            std::fs::rename(root.join("a"), root.join("old")).unwrap();
            std::fs::write(root.join("a"), b"replacement").unwrap();
        });
        assert_eq!(entry.observation, Observation::UnstableDuringRead);
        assert!(entry.sha256.is_none());
    }
}
