//! Dysnomia filesystem operations provided by PAL ABI v1.
//!
//! `File` keeps a path and seek cursor; `read` and `write` pass an explicit
//! offset. Paths use Dysnomia's byte-based `OsStr` encoding.
//!
//! Symlinks, hard links, permissions, timestamps, truncation, explicit directory
//! creation, file locks, and handle duplication are unsupported.

use crate::ffi::OsString;
use crate::fmt;
use crate::fs::TryLockError;
use crate::hash::Hash;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, SeekFrom};
use crate::path::{Path, PathBuf};
use crate::sync::atomic::{AtomicU64, Ordering};
pub use crate::sys::fs::common::Dir;
use crate::sys::os_str::Buf;
use crate::sys::pal::{abi, abi_bool, count_result, invalid_data, status_result};
use crate::sys::time::SystemTime;
use crate::sys::{FromInner, unsupported};
use crate::vec::Vec;

/// The raw path bytes used by the Dysnomia ABI.
fn path_bytes(p: &Path) -> &[u8] {
    p.as_os_str().as_encoded_bytes()
}

/// Build an `OsString` from raw name bytes, losslessly (the args-arm posture).
fn os_string(bytes: &[u8]) -> OsString {
    OsString::from_inner(Buf::from_inner(bytes.to_vec()))
}

pub struct File {
    path: PathBuf,
    /// The client-side seek cursor. An atomic so
    /// `File` stays `Send + Sync` like every other platform's `File`.
    pos: AtomicU64,
}

/// File attributes exposed by the current ABI.
#[derive(Clone)]
pub struct FileAttr {
    size: u64,
    is_dir: bool,
}

pub struct ReadDir {
    /// The listed directory, for `DirEntry::path`.
    parent: PathBuf,
    /// The directory-iteration handle, released on `Drop`.
    handle: u64,
}

pub struct DirEntry {
    parent: PathBuf,
    name: Vec<u8>,
    is_dir: bool,
    size: u64,
}

#[derive(Clone, Debug)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct FileTimes {}

/// The store has no mode bits — authority is the capability rights mask,
/// so a file is never "read-only" in the POSIX sense. Carried so the type exists.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FilePermissions {
    readonly: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FileType {
    is_dir: bool,
}

#[derive(Debug)]
pub struct DirBuilder {}

impl FileAttr {
    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn perm(&self) -> FilePermissions {
        FilePermissions { readonly: false }
    }

    pub fn file_type(&self) -> FileType {
        FileType { is_dir: self.is_dir }
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        unsupported()
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        // No atime in the store.
        unsupported()
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        unsupported()
    }
}

impl FilePermissions {
    pub fn readonly(&self) -> bool {
        self.readonly
    }

    pub fn set_readonly(&mut self, readonly: bool) {
        // A local flag only; applying it (`set_perm`) is Unsupported — the store has
        // no mode bits.
        self.readonly = readonly;
    }
}

impl FileTimes {
    pub fn set_accessed(&mut self, _t: SystemTime) {}
    pub fn set_modified(&mut self, _t: SystemTime) {}
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn is_file(&self) -> bool {
        !self.is_dir
    }

    pub fn is_symlink(&self) -> bool {
        // No symlinks in the store.
        false
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadDir").field("parent", &self.parent).finish_non_exhaustive()
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        // The ABI limits one entry name to 255 bytes.
        let mut name = [0u8; 255];
        let mut metadata =
            abi::DirectoryEntryMetadata { size: 0, name_len: 0, is_dir: 0, reserved: [0; 5] };
        let status = unsafe {
            abi::__dysnomia_pal_v1_fs_readdir_next(
                self.handle,
                name.as_mut_ptr(),
                name.len() as u64,
                &mut metadata,
            )
        };
        match status {
            0 => {
                let name_len = match usize::try_from(metadata.name_len) {
                    Ok(name_len) if name_len <= name.len() => name_len,
                    _ => return Some(Err(invalid_data("invalid directory entry name length"))),
                };
                if metadata.reserved != [0; 5] {
                    return Some(Err(invalid_data("nonzero reserved directory metadata")));
                }
                let is_dir = match abi_bool(metadata.is_dir) {
                    Ok(is_dir) => is_dir,
                    Err(error) => return Some(Err(error)),
                };
                Some(Ok(DirEntry {
                    parent: self.parent.clone(),
                    name: name[..name_len].to_vec(),
                    is_dir,
                    size: metadata.size,
                }))
            }
            1 => None,
            status => Some(Err(status_result(status).unwrap_err())),
        }
    }
}

impl Drop for ReadDir {
    fn drop(&mut self) {
        // SAFETY: the handle came from `_open` and is closed exactly once.
        unsafe { abi::__dysnomia_pal_v1_fs_readdir_close(self.handle) };
    }
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.parent.join(os_string(&self.name))
    }

    pub fn file_name(&self) -> OsString {
        os_string(&self.name)
    }

    pub fn metadata(&self) -> io::Result<FileAttr> {
        Ok(FileAttr { size: self.size, is_dir: self.is_dir })
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(FileType { is_dir: self.is_dir })
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
        }
    }

    pub fn read(&mut self, read: bool) {
        self.read = read;
    }
    pub fn write(&mut self, write: bool) {
        self.write = write;
    }
    pub fn append(&mut self, append: bool) {
        self.append = append;
    }
    pub fn truncate(&mut self, truncate: bool) {
        self.truncate = truncate;
    }
    pub fn create(&mut self, create: bool) {
        self.create = create;
    }
    pub fn create_new(&mut self, create_new: bool) {
        self.create_new = create_new;
    }
}

fn stat_size(bytes: &[u8]) -> io::Result<u64> {
    let mut size = 0;
    let status =
        unsafe { abi::__dysnomia_pal_v1_fs_stat(bytes.as_ptr(), bytes.len() as u64, &mut size) };
    status_result(status)?;
    Ok(size)
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let bytes = path_bytes(path);
        if opts.create_new {
            // Must not already exist.
            match stat_size(bytes) {
                Ok(_) => return Err(io::Error::from(io::ErrorKind::AlreadyExists)),
                Err(error) if error.raw_os_error().is_none() => return Err(error),
                Err(_) => {}
            }
        } else if !opts.write && !opts.append && !opts.create && !opts.truncate {
            // A pure read open: the file must exist (surface its error otherwise).
            stat_size(bytes)?;
        }
        // Truncate: emulate by dropping any existing content — creation is a side
        // effect of the first `Write`, so a fresh path is `NotFound`
        // (ignored). Skipped for `create_new` (nothing to truncate).
        if opts.truncate && !opts.create_new {
            // SAFETY: plain marshalling call.
            let status =
                unsafe { abi::__dysnomia_pal_v1_fs_unlink(bytes.as_ptr(), bytes.len() as u64) };
            if status > 0 {
                return Err(status_result(status).unwrap_err());
            }
        }
        // Append starts the cursor at end-of-file.
        let pos = if opts.append {
            match stat_size(bytes) {
                Ok(size) => size,
                Err(error) if error.raw_os_error().is_none() => return Err(error),
                Err(_) => 0,
            }
        } else {
            0
        };
        Ok(File { path: path.to_path_buf(), pos: AtomicU64::new(pos) })
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        stat_attr(&self.path)
    }

    pub fn fsync(&self) -> io::Result<()> {
        // SAFETY: this ABI function takes no pointers.
        status_result(unsafe { abi::__dysnomia_pal_v1_fs_sync() })
    }

    pub fn datasync(&self) -> io::Result<()> {
        self.fsync()
    }

    pub fn lock(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn lock_shared(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn try_lock(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(unsupported::<()>().unwrap_err()))
    }

    pub fn try_lock_shared(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(unsupported::<()>().unwrap_err()))
    }

    pub fn unlock(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn truncate(&self, _size: u64) -> io::Result<()> {
        // No `set_len`/truncate op in the store.
        unsupported()
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let off = self.pos.load(Ordering::Relaxed);
        // SAFETY: plain marshalling call; `buf` outlives it.
        let path = path_bytes(&self.path);
        let mut count = 0;
        let status = unsafe {
            abi::__dysnomia_pal_v1_fs_read(
                path.as_ptr(),
                path.len() as u64,
                off,
                buf.as_mut_ptr(),
                buf.len() as u64,
                &mut count,
            )
        };
        let n = count_result(status, count, buf.len())?;
        self.pos.store(off + n as u64, Ordering::Relaxed);
        Ok(n)
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        for b in bufs {
            if !b.is_empty() {
                return self.read(b);
            }
        }
        Ok(0)
    }

    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn read_buf(&self, mut cursor: BorrowedCursor<'_>) -> io::Result<()> {
        let mut tmp = [0u8; 512];
        let want = cursor.capacity().min(tmp.len());
        if want == 0 {
            return Ok(());
        }
        let n = self.read(&mut tmp[..want])?;
        cursor.append(&tmp[..n]);
        Ok(())
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let off = self.pos.load(Ordering::Relaxed);
        // SAFETY: plain marshalling call; `buf` outlives it.
        let path = path_bytes(&self.path);
        let mut count = 0;
        let status = unsafe {
            abi::__dysnomia_pal_v1_fs_write(
                path.as_ptr(),
                path.len() as u64,
                off,
                buf.as_ptr(),
                buf.len() as u64,
                &mut count,
            )
        };
        let n = count_result(status, count, buf.len())?;
        self.pos.store(off + n as u64, Ordering::Relaxed);
        Ok(n)
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        for b in bufs {
            if !b.is_empty() {
                return self.write(b);
            }
        }
        Ok(0)
    }

    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn flush(&self) -> io::Result<()> {
        // Writes are synchronous through the session; durability is `fsync`/`Sync`.
        Ok(())
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        let new = match pos {
            SeekFrom::Start(n) => n as i128,
            SeekFrom::Current(d) => self.pos.load(Ordering::Relaxed) as i128 + d as i128,
            SeekFrom::End(d) => stat_size(path_bytes(&self.path))? as i128 + d as i128,
        };
        if new < 0 {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        let new = new as u64;
        self.pos.store(new, Ordering::Relaxed);
        Ok(new)
    }

    pub fn size(&self) -> Option<io::Result<u64>> {
        Some(self.file_attr().map(|a| a.size()))
    }

    pub fn tell(&self) -> io::Result<u64> {
        Ok(self.pos.load(Ordering::Relaxed))
    }

    pub fn duplicate(&self) -> io::Result<File> {
        // A dup would share the cursor (an fd-level offset); the store has no such
        // object, so this is Unsupported rather than a silently-diverging copy.
        unsupported()
    }

    pub fn set_permissions(&self, _perm: FilePermissions) -> io::Result<()> {
        unsupported()
    }

    pub fn set_times(&self, _times: FileTimes) -> io::Result<()> {
        unsupported()
    }
}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    pub fn mkdir(&self, _p: &Path) -> io::Result<()> {
        // Directories are created as a side effect of writing a file beneath them;
        // there is no explicit empty-directory creation.
        unsupported()
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File").field("path", &self.path).finish_non_exhaustive()
    }
}

pub fn readdir(p: &Path) -> io::Result<ReadDir> {
    let path = path_bytes(p);
    let mut handle = 0;
    let status = unsafe {
        abi::__dysnomia_pal_v1_fs_readdir_open(path.as_ptr(), path.len() as u64, &mut handle)
    };
    status_result(status)?;
    Ok(ReadDir { parent: p.to_path_buf(), handle })
}

pub fn unlink(p: &Path) -> io::Result<()> {
    // SAFETY: plain marshalling call.
    let path = path_bytes(p);
    status_result(unsafe { abi::__dysnomia_pal_v1_fs_unlink(path.as_ptr(), path.len() as u64) })
}

pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    // SAFETY: plain marshalling call.
    let old = path_bytes(old);
    let new = path_bytes(new);
    status_result(unsafe {
        abi::__dysnomia_pal_v1_fs_rename(
            old.as_ptr(),
            old.len() as u64,
            new.as_ptr(),
            new.len() as u64,
        )
    })
}

pub fn set_perm(_p: &Path, _perm: FilePermissions) -> io::Result<()> {
    // Authority is the cap rights mask, not mode bits.
    unsupported()
}

pub fn set_times(_p: &Path, _times: FileTimes) -> io::Result<()> {
    unsupported()
}

pub fn set_times_nofollow(_p: &Path, _times: FileTimes) -> io::Result<()> {
    unsupported()
}

pub fn rmdir(_p: &Path) -> io::Result<()> {
    // No explicit directory removal op (directories are implicit).
    unsupported()
}

pub fn remove_dir_all(path: &Path) -> io::Result<()> {
    crate::sys::fs::common::remove_dir_all(path)
}

pub fn exists(path: &Path) -> io::Result<bool> {
    crate::sys::fs::common::exists(path)
}

pub fn readlink(_p: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn symlink(_original: &Path, _link: &Path) -> io::Result<()> {
    unsupported()
}

pub fn link(_src: &Path, _dst: &Path) -> io::Result<()> {
    unsupported()
}

fn stat_attr(p: &Path) -> io::Result<FileAttr> {
    let path = path_bytes(p);
    let mut metadata = abi::FileMetadata { size: 0, is_dir: 0, reserved: [0; 5] };
    let status = unsafe {
        abi::__dysnomia_pal_v1_fs_metadata(path.as_ptr(), path.len() as u64, &mut metadata)
    };
    status_result(status)?;
    if metadata.reserved != [0; 5] {
        return Err(invalid_data("nonzero reserved file metadata"));
    }
    Ok(FileAttr { size: metadata.size, is_dir: abi_bool(metadata.is_dir)? })
}

pub fn stat(p: &Path) -> io::Result<FileAttr> {
    stat_attr(p)
}

pub fn lstat(p: &Path) -> io::Result<FileAttr> {
    // No symlinks, so lstat == stat.
    stat_attr(p)
}

pub fn canonicalize(_p: &Path) -> io::Result<PathBuf> {
    // No ambient root/`..` resolution service (handle-relative).
    unsupported()
}

pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    crate::sys::fs::common::copy(from, to)
}
