//! The eunomia filesystem arm (std-port 4.1): a thin marshalling shell over the
//! storaged session client in the seam crate `eunomia-sys`. Every real op is a
//! one-line delegation to a `__eunomia_fs_*` `extern "Rust"` symbol (see
//! `sys/pal/eunomia/mod.rs` for why the seam is a link-time symbol, not a direct
//! call); this file holds only std's `File`/`ReadDir`/`FileAttr` bookkeeping — the
//! client cursor and the path/entry plumbing — never any protocol logic.
//!
//! `File = (path, cursor)`: storaged is offset-stateless, so the seek cursor lives
//! here and `read`/`write` pass an explicit offset. eunomia's `OsStr` is bytes
//! (rev2§4.9), so a path crosses the seam as its raw encoded bytes; the seam splits
//! it into tree components (the 4.2 seam). A `< 0` return is a raw fs code the arm
//! wraps with `io::Error::from_raw_os_error` (kind via `decode_error_kind`).
//!
//! Surface that is `Unsupported` by construction (rev2§4.9 has none of it): symlinks
//! / hard links / readlink / canonicalize; permissions / `chmod`; `set_times`;
//! `truncate`/`set_len`; `mkdir` (creation is a side effect of `Write`); file locks;
//! `duplicate`. Metadata is minimal (size + file/dir); mtime and the fuller
//! `ErrorCode`→`ErrorKind` table are 4.3.

use crate::ffi::{OsStr, OsString};
use crate::fmt;
use crate::fs::TryLockError;
use crate::hash::{Hash, Hasher};
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, SeekFrom};
use crate::path::{Path, PathBuf};
use crate::sync::atomic::{AtomicU64, Ordering};
pub use crate::sys::fs::common::Dir;
use crate::sys::os_str::Buf;
use crate::sys::time::SystemTime;
use crate::sys::{FromInner, unsupported};
use crate::vec::Vec;

// Provided by the seam crate `eunomia-sys` (its `#[no_mangle]` `pal.rs` shims over
// `eunomia_sys::fs`). Raw path *bytes* cross the seam; a `< 0` return is a raw fs
// code. `readdir` returns the flat entry buffer decoded by `parse_listing` below.
unsafe extern "Rust" {
    fn __eunomia_fs_read(path: &[u8], offset: u64, buf: &mut [u8]) -> i64;
    fn __eunomia_fs_write(path: &[u8], offset: u64, data: &[u8]) -> i64;
    fn __eunomia_fs_stat(path: &[u8]) -> i64;
    fn __eunomia_fs_rename(from: &[u8], to: &[u8]) -> i64;
    fn __eunomia_fs_unlink(path: &[u8]) -> i64;
    fn __eunomia_fs_sync() -> i64;
    fn __eunomia_fs_readdir(path: &[u8]) -> Vec<u8>;
}

/// The raw path bytes eunomia sends over the seam (its `OsStr` is bytes, rev2§4.9).
fn path_bytes(p: &Path) -> &[u8] {
    p.as_os_str().as_encoded_bytes()
}

/// Build an `OsString` from raw name bytes, losslessly (the args-arm posture).
fn os_string(bytes: &[u8]) -> OsString {
    OsString::from_inner(Buf::from_inner(bytes.to_vec()))
}

/// A `< 0` seam code as an `io::Error` (its kind flows through `decode_error_kind`).
/// The fs codes fit in `i32` by construction (`eunomia_sys::io_error` band).
fn err(code: i64) -> io::Error {
    io::Error::from_raw_os_error(code as i32)
}

pub struct File {
    path: PathBuf,
    /// The client-side seek cursor (storaged is offset-stateless). An atomic so
    /// `File` stays `Send + Sync` like every other platform's `File`.
    pos: AtomicU64,
}

/// Minimal file attributes (std-port 4.1): the content size and whether the entry
/// is a directory. mtime/atime and richer types are 4.3.
#[derive(Clone)]
pub struct FileAttr {
    size: u64,
    is_dir: bool,
}

pub struct ReadDir {
    /// The listed directory, for `DirEntry::path`.
    parent: PathBuf,
    entries: Vec<RawEntry>,
    idx: usize,
}

struct RawEntry {
    name: Vec<u8>,
    is_dir: bool,
    size: u64,
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

/// The store has no mode bits — authority is the capability rights mask (rev2§2.3),
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
        // mtime is a mandatory rev2§4.9 field absent from the current wire protocol
        // (a deferred storage-wire extension); unsupported for now.
        unsupported()
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        // No atime in the store (rev2§4.9).
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
        // no mode bits (rev2§2.3).
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
        // No symlinks in the store (rev2§4.9).
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
        let e = self.entries.get(self.idx)?;
        self.idx += 1;
        Some(Ok(DirEntry {
            parent: self.parent.clone(),
            name: e.name.clone(),
            is_dir: e.is_dir,
            size: e.size,
        }))
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

/// The size of an existing file, or the raw seam code (`< 0`, `ERR_FS_NOT_FOUND` if
/// absent). `Stat` reads content length, so it answers for files.
fn stat_size(bytes: &[u8]) -> i64 {
    // SAFETY: the seam symbol is a plain marshalling call with no pointer contract
    // beyond the borrowed slice, which outlives the call.
    unsafe { __eunomia_fs_stat(bytes) }
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let bytes = path_bytes(path);
        if opts.create_new {
            // Must not already exist.
            if stat_size(bytes) >= 0 {
                return Err(io::Error::from(io::ErrorKind::AlreadyExists));
            }
        } else if !opts.write && !opts.append && !opts.create && !opts.truncate {
            // A pure read open: the file must exist (surface its error otherwise).
            let r = stat_size(bytes);
            if r < 0 {
                return Err(err(r));
            }
        }
        // Truncate: emulate by dropping any existing content — creation is a side
        // effect of the first `Write` (rev2§4.9), so a fresh path is `NotFound`
        // (ignored). Skipped for `create_new` (nothing to truncate).
        if opts.truncate && !opts.create_new {
            // SAFETY: plain marshalling call.
            let _ = unsafe { __eunomia_fs_unlink(bytes) };
        }
        // Append starts the cursor at end-of-file.
        let pos = if opts.append {
            let r = stat_size(bytes);
            if r >= 0 { r as u64 } else { 0 }
        } else {
            0
        };
        Ok(File { path: path.to_path_buf(), pos: AtomicU64::new(pos) })
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        let r = stat_size(path_bytes(&self.path));
        if r < 0 {
            return Err(err(r));
        }
        Ok(FileAttr { size: r as u64, is_dir: false })
    }

    pub fn fsync(&self) -> io::Result<()> {
        // storaged syncs the whole ref (rev2§4.4), a superset of one file's fsync.
        // SAFETY: plain marshalling call.
        let r = unsafe { __eunomia_fs_sync() };
        if r == 0 { Ok(()) } else { Err(err(r)) }
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
        // No `set_len`/truncate op in the store (rev2§4.9).
        unsupported()
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let off = self.pos.load(Ordering::Relaxed);
        // SAFETY: plain marshalling call; `buf` outlives it.
        let r = unsafe { __eunomia_fs_read(path_bytes(&self.path), off, buf) };
        if r < 0 {
            return Err(err(r));
        }
        let n = r as usize;
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

    pub fn read_buf(&self, mut cursor: BorrowedCursor<'_, u8>) -> io::Result<()> {
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
        let r = unsafe { __eunomia_fs_write(path_bytes(&self.path), off, buf) };
        if r < 0 {
            return Err(err(r));
        }
        let n = r as usize;
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
            SeekFrom::End(d) => {
                let r = stat_size(path_bytes(&self.path));
                if r < 0 {
                    return Err(err(r));
                }
                r as i128 + d as i128
            }
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
        // Directories are created as a side effect of writing a file beneath them
        // (rev2§4.9); there is no explicit empty-directory creation.
        unsupported()
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File").field("path", &self.path).finish_non_exhaustive()
    }
}

// ── The readdir flat buffer (shared with `eunomia_sys::fs`) ──
// byte 0: 0 = ok (entries follow), 1 = error (8-byte i64 LE code follows)
// each entry: [kind: u8 (0=file,1=dir)][size: u64 LE][name_len: u16 LE][name…]
fn parse_listing(parent: &Path, buf: &[u8]) -> io::Result<ReadDir> {
    let mut it = buf.iter().copied();
    match it.next() {
        Some(0) => {}
        Some(1) => {
            let mut code = [0u8; 8];
            for c in &mut code {
                *c = it.next().unwrap_or(0);
            }
            return Err(err(i64::from_le_bytes(code)));
        }
        // A malformed/empty buffer (the seam always tags its output) — treat as a
        // transport error rather than an empty listing.
        _ => return Err(io::Error::from(io::ErrorKind::InvalidData)),
    }
    let mut rest = &buf[1..];
    let mut entries = Vec::new();
    while !rest.is_empty() {
        // kind(1) + size(8) + name_len(2) = 11-byte fixed head.
        if rest.len() < 11 {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        let is_dir = rest[0] == 1;
        let size = u64::from_le_bytes(rest[1..9].try_into().unwrap());
        let name_len = u16::from_le_bytes(rest[9..11].try_into().unwrap()) as usize;
        rest = &rest[11..];
        if rest.len() < name_len {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        entries.push(RawEntry { name: rest[..name_len].to_vec(), is_dir, size });
        rest = &rest[name_len..];
    }
    Ok(ReadDir { parent: parent.to_path_buf(), entries, idx: 0 })
}

pub fn readdir(p: &Path) -> io::Result<ReadDir> {
    // SAFETY: plain marshalling call; the returned `Vec<u8>` is owned across the seam.
    let buf = unsafe { __eunomia_fs_readdir(path_bytes(p)) };
    parse_listing(p, &buf)
}

pub fn unlink(p: &Path) -> io::Result<()> {
    // SAFETY: plain marshalling call.
    let r = unsafe { __eunomia_fs_unlink(path_bytes(p)) };
    if r == 0 { Ok(()) } else { Err(err(r)) }
}

pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    // SAFETY: plain marshalling call.
    let r = unsafe { __eunomia_fs_rename(path_bytes(old), path_bytes(new)) };
    if r == 0 { Ok(()) } else { Err(err(r)) }
}

pub fn set_perm(_p: &Path, _perm: FilePermissions) -> io::Result<()> {
    // Authority is the cap rights mask, not mode bits (rev2§2.3).
    unsupported()
}

pub fn set_times(_p: &Path, _times: FileTimes) -> io::Result<()> {
    unsupported()
}

pub fn set_times_nofollow(_p: &Path, _times: FileTimes) -> io::Result<()> {
    unsupported()
}

pub fn rmdir(_p: &Path) -> io::Result<()> {
    // No explicit directory removal op (directories are implicit, rev2§4.9).
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
    let r = stat_size(path_bytes(p));
    if r < 0 {
        return Err(err(r));
    }
    Ok(FileAttr { size: r as u64, is_dir: false })
}

pub fn stat(p: &Path) -> io::Result<FileAttr> {
    stat_attr(p)
}

pub fn lstat(p: &Path) -> io::Result<FileAttr> {
    // No symlinks, so lstat == stat (rev2§4.9).
    stat_attr(p)
}

pub fn canonicalize(_p: &Path) -> io::Result<PathBuf> {
    // No ambient root/`..` resolution service (handle-relative, rev2§4.9).
    unsupported()
}

pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    crate::sys::fs::common::copy(from, to)
}
