use crate::path::PathBuf;

// The writable scratch directory. Resolved from the `TMPDIR` environment
// variable the spawner delivers in the startup block, falling back to
// the conventional `/tmp` — the unix policy. There is no ambient filesystem namespace
// (names are handle-relative), so this is a name a program hands to its
// storage root, not an OS-resolved absolute path; `temp_dir()` does not touch a
// filesystem, so it never fails. This replaces the `unsupported` arm, whose
// `temp_dir()` panics — an infallible std function must return a path.
pub fn temp_dir() -> PathBuf {
    crate::env::var_os("TMPDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}
