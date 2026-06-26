// SPDX-License-Identifier: 0BSD

//! Canonical raw declarations for version 1 of the Dysnomia PAL ABI.

#![allow(dead_code)]
#![deny(improper_ctypes, improper_ctypes_definitions)]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BorrowedBytes {
    pub ptr: *const u8,
    pub len: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FileMetadata {
    pub size: u64,
    pub is_dir: u32,
    pub reserved: [u32; 5],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DirectoryEntryMetadata {
    pub size: u64,
    pub name_len: u64,
    pub is_dir: u32,
    pub reserved: [u32; 5],
}

pub(crate) type ThreadEntry = unsafe extern "C" fn(u64) -> !;
pub(crate) type TlsDestructor = unsafe extern "C" fn(*mut u8);

pub(crate) type ThreadExitFn = unsafe extern "C" fn(u64) -> !;
pub(crate) type AllocFn = unsafe extern "C" fn(u64, u64) -> *mut u8;
pub(crate) type DeallocFn = unsafe extern "C" fn(*mut u8, u64, u64);
pub(crate) type ByteTableFn = unsafe extern "C" fn(*mut *const BorrowedBytes, *mut u64) -> i32;
pub(crate) type ConstIoFn = unsafe extern "C" fn(*const u8, u64, *mut u64) -> i32;
pub(crate) type MutIoFn = unsafe extern "C" fn(*mut u8, u64, *mut u64) -> i32;
pub(crate) type IoClassifyFn = unsafe extern "C" fn(i32) -> i32;
pub(crate) type IoMessageFn = unsafe extern "C" fn(i32) -> BorrowedBytes;
pub(crate) type FsReadFn = unsafe extern "C" fn(*const u8, u64, u64, *mut u8, u64, *mut u64) -> i32;
pub(crate) type FsWriteFn =
    unsafe extern "C" fn(*const u8, u64, u64, *const u8, u64, *mut u64) -> i32;
pub(crate) type FsStatFn = unsafe extern "C" fn(*const u8, u64, *mut u64) -> i32;
pub(crate) type FsMetadataFn = unsafe extern "C" fn(*const u8, u64, *mut FileMetadata) -> i32;
pub(crate) type FsRenameFn = unsafe extern "C" fn(*const u8, u64, *const u8, u64) -> i32;
pub(crate) type FsPathFn = unsafe extern "C" fn(*const u8, u64) -> i32;
pub(crate) type StatusFn = unsafe extern "C" fn() -> i32;
pub(crate) type FsReaddirOpenFn = unsafe extern "C" fn(*const u8, u64, *mut u64) -> i32;
pub(crate) type FsReaddirNextFn =
    unsafe extern "C" fn(u64, *mut u8, u64, *mut DirectoryEntryMetadata) -> i32;
pub(crate) type HandleFn = unsafe extern "C" fn(u64);
pub(crate) type ThreadSpawnFn = unsafe extern "C" fn(ThreadEntry, u64, u64, *mut u64) -> i32;
pub(crate) type ThreadJoinFn = unsafe extern "C" fn(u64) -> i32;
pub(crate) type VoidFn = unsafe extern "C" fn();
pub(crate) type U64Fn = unsafe extern "C" fn(u64);
pub(crate) type TlsCreateFn = unsafe extern "C" fn(TlsDestructor, u32, *mut u64) -> i32;
pub(crate) type TlsGetFn = unsafe extern "C" fn(u64) -> *mut u8;
pub(crate) type TlsSetFn = unsafe extern "C" fn(u64, *mut u8);
pub(crate) type FutexWaitFn = unsafe extern "C" fn(*const u32, u32, u64) -> u32;
pub(crate) type FutexWakeFn = unsafe extern "C" fn(*const u32) -> u32;
pub(crate) type FutexWakeAllFn = unsafe extern "C" fn(*const u32);
pub(crate) type ClockFn = unsafe extern "C" fn() -> i64;
pub(crate) type FillBytesFn = unsafe extern "C" fn(*mut u8, u64);

unsafe extern "C" {
    pub(crate) fn __dysnomia_pal_v1_thread_exit(code: u64) -> !;

    pub(crate) fn __dysnomia_pal_v1_alloc(size: u64, align: u64) -> *mut u8;
    pub(crate) fn __dysnomia_pal_v1_dealloc(ptr: *mut u8, size: u64, align: u64);

    pub(crate) fn __dysnomia_pal_v1_argv(
        entries_out: *mut *const BorrowedBytes,
        count_out: *mut u64,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_env(
        entries_out: *mut *const BorrowedBytes,
        count_out: *mut u64,
    ) -> i32;

    pub(crate) fn __dysnomia_pal_v1_stdout_write(
        buf: *const u8,
        len: u64,
        count_out: *mut u64,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_stderr_write(
        buf: *const u8,
        len: u64,
        count_out: *mut u64,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_stdin_read(buf: *mut u8, len: u64, count_out: *mut u64) -> i32;
    pub(crate) fn __dysnomia_pal_v1_stdio_write(
        buf: *const u8,
        len: u64,
        count_out: *mut u64,
    ) -> i32;

    pub(crate) fn __dysnomia_pal_v1_io_classify(code: i32) -> i32;
    pub(crate) fn __dysnomia_pal_v1_io_message(code: i32) -> BorrowedBytes;

    pub(crate) fn __dysnomia_pal_v1_fs_read(
        path: *const u8,
        path_len: u64,
        offset: u64,
        buf: *mut u8,
        buf_len: u64,
        count_out: *mut u64,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_fs_write(
        path: *const u8,
        path_len: u64,
        offset: u64,
        data: *const u8,
        data_len: u64,
        count_out: *mut u64,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_fs_stat(
        path: *const u8,
        path_len: u64,
        size_out: *mut u64,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_fs_metadata(
        path: *const u8,
        path_len: u64,
        metadata_out: *mut FileMetadata,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_fs_rename(
        from: *const u8,
        from_len: u64,
        to: *const u8,
        to_len: u64,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_fs_unlink(path: *const u8, path_len: u64) -> i32;
    pub(crate) fn __dysnomia_pal_v1_fs_sync() -> i32;
    pub(crate) fn __dysnomia_pal_v1_fs_readdir_open(
        path: *const u8,
        path_len: u64,
        handle_out: *mut u64,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_fs_readdir_next(
        handle: u64,
        name_buf: *mut u8,
        name_buf_len: u64,
        metadata_out: *mut DirectoryEntryMetadata,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_fs_readdir_close(handle: u64);

    pub(crate) fn __dysnomia_pal_v1_thread_spawn(
        entry: ThreadEntry,
        stack_size: u64,
        arg: u64,
        handle_out: *mut u64,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_thread_join(handle: u64) -> i32;
    pub(crate) fn __dysnomia_pal_v1_thread_yield();
    pub(crate) fn __dysnomia_pal_v1_thread_sleep(nanos: u64);
    pub(crate) fn __dysnomia_pal_v1_tls_init_thread();
    pub(crate) fn __dysnomia_pal_v1_tls_run_dtors();
    pub(crate) fn __dysnomia_pal_v1_tls_free_thread();
    pub(crate) fn __dysnomia_pal_v1_tls_create(
        dtor: TlsDestructor,
        dtor_present: u32,
        key_out: *mut u64,
    ) -> i32;
    pub(crate) fn __dysnomia_pal_v1_tls_get(key: u64) -> *mut u8;
    pub(crate) fn __dysnomia_pal_v1_tls_set(key: u64, value: *mut u8);
    pub(crate) fn __dysnomia_pal_v1_tls_destroy(key: u64);

    pub(crate) fn __dysnomia_pal_v1_futex_wait(
        futex: *const u32,
        expected: u32,
        timeout_ns: u64,
    ) -> u32;
    pub(crate) fn __dysnomia_pal_v1_futex_wake(futex: *const u32) -> u32;
    pub(crate) fn __dysnomia_pal_v1_futex_wake_all(futex: *const u32);

    pub(crate) fn __dysnomia_pal_v1_mono_ns() -> i64;
    pub(crate) fn __dysnomia_pal_v1_wall_ns() -> i64;
    pub(crate) fn __dysnomia_pal_v1_fill_bytes(bytes: *mut u8, len: u64);
}

const _: ThreadExitFn = __dysnomia_pal_v1_thread_exit;
const _: AllocFn = __dysnomia_pal_v1_alloc;
const _: DeallocFn = __dysnomia_pal_v1_dealloc;
const _: ByteTableFn = __dysnomia_pal_v1_argv;
const _: ByteTableFn = __dysnomia_pal_v1_env;
const _: ConstIoFn = __dysnomia_pal_v1_stdout_write;
const _: ConstIoFn = __dysnomia_pal_v1_stderr_write;
const _: MutIoFn = __dysnomia_pal_v1_stdin_read;
const _: ConstIoFn = __dysnomia_pal_v1_stdio_write;
const _: IoClassifyFn = __dysnomia_pal_v1_io_classify;
const _: IoMessageFn = __dysnomia_pal_v1_io_message;
const _: FsReadFn = __dysnomia_pal_v1_fs_read;
const _: FsWriteFn = __dysnomia_pal_v1_fs_write;
const _: FsStatFn = __dysnomia_pal_v1_fs_stat;
const _: FsMetadataFn = __dysnomia_pal_v1_fs_metadata;
const _: FsRenameFn = __dysnomia_pal_v1_fs_rename;
const _: FsPathFn = __dysnomia_pal_v1_fs_unlink;
const _: StatusFn = __dysnomia_pal_v1_fs_sync;
const _: FsReaddirOpenFn = __dysnomia_pal_v1_fs_readdir_open;
const _: FsReaddirNextFn = __dysnomia_pal_v1_fs_readdir_next;
const _: HandleFn = __dysnomia_pal_v1_fs_readdir_close;
const _: ThreadSpawnFn = __dysnomia_pal_v1_thread_spawn;
const _: ThreadJoinFn = __dysnomia_pal_v1_thread_join;
const _: VoidFn = __dysnomia_pal_v1_thread_yield;
const _: U64Fn = __dysnomia_pal_v1_thread_sleep;
const _: VoidFn = __dysnomia_pal_v1_tls_init_thread;
const _: VoidFn = __dysnomia_pal_v1_tls_run_dtors;
const _: VoidFn = __dysnomia_pal_v1_tls_free_thread;
const _: TlsCreateFn = __dysnomia_pal_v1_tls_create;
const _: TlsGetFn = __dysnomia_pal_v1_tls_get;
const _: TlsSetFn = __dysnomia_pal_v1_tls_set;
const _: HandleFn = __dysnomia_pal_v1_tls_destroy;
const _: FutexWaitFn = __dysnomia_pal_v1_futex_wait;
const _: FutexWakeFn = __dysnomia_pal_v1_futex_wake;
const _: FutexWakeAllFn = __dysnomia_pal_v1_futex_wake_all;
const _: ClockFn = __dysnomia_pal_v1_mono_ns;
const _: ClockFn = __dysnomia_pal_v1_wall_ns;
const _: FillBytesFn = __dysnomia_pal_v1_fill_bytes;

#[cfg(target_arch = "aarch64")]
const _: () = {
    use core::mem::{align_of, offset_of, size_of};

    assert!(offset_of!(BorrowedBytes, ptr) == 0);
    assert!(offset_of!(BorrowedBytes, len) == 8);
    assert!(size_of::<BorrowedBytes>() == 16);
    assert!(align_of::<BorrowedBytes>() == 8);

    assert!(offset_of!(FileMetadata, size) == 0);
    assert!(offset_of!(FileMetadata, is_dir) == 8);
    assert!(offset_of!(FileMetadata, reserved) == 12);
    assert!(size_of::<FileMetadata>() == 32);
    assert!(align_of::<FileMetadata>() == 8);

    assert!(offset_of!(DirectoryEntryMetadata, size) == 0);
    assert!(offset_of!(DirectoryEntryMetadata, name_len) == 8);
    assert!(offset_of!(DirectoryEntryMetadata, is_dir) == 16);
    assert!(offset_of!(DirectoryEntryMetadata, reserved) == 20);
    assert!(size_of::<DirectoryEntryMetadata>() == 40);
    assert!(align_of::<DirectoryEntryMetadata>() == 8);
};
