// Provided by the seam crate `eunomia-sys` (see `sys/pal/eunomia/mod.rs`): fill
// `bytes` from the per-process DRBG (xoshiro256** over the `NAME_RANDOM_SEED`
// grant, std-port 3.4). All logic lives in the seam (`urt::random` via
// `eunomia_sys::random`); this arm only delegates, the `sys/stdio/eunomia.rs`
// pattern with a `&mut` slice. `hashmap_random_keys` is the generic one in
// `mod.rs` (it calls `fill_bytes`), so only `fill_bytes` is defined here — the
// `motor` shape.
//
// If the process was not granted a seed, the seam loudly aborts on the first
// call (the `urt::random` no-seed posture, the `SystemTime`-without-time-grant
// precedent) rather than returning silently-predictable bytes.
unsafe extern "Rust" {
    fn __eunomia_fill_bytes(bytes: &mut [u8]);
}

pub fn fill_bytes(bytes: &mut [u8]) {
    // SAFETY: the seam fn is a pure delegation — it writes `bytes` (a valid
    // mutable slice) from the process DRBG and allocates nothing.
    unsafe { __eunomia_fill_bytes(bytes) }
}
