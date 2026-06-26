use crate::time::Duration;

// Provided by the seam crate `dysnomia-sys` (see `sys/pal/dysnomia/mod.rs` for why these
// are `extern "Rust"` symbols rather than direct calls): monotonic nanoseconds for
// `Instant` (the CNTVCT/CNTFRQ virtual counter via urt's Verus-verified `utc_ns_at`, no
// `"time"` grant needed) and wall-clock nanoseconds since the Unix epoch for
// `SystemTime` (the seam panics if no `"time"` grant was
// attached). Both return `i64` ns; this arm only wraps them into std's `Duration`-based
// types and re-establishes `Duration`'s non-negativity precondition at the boundary
// (`ns.max(0) as u64` — the §11 inverse-leak guard). All clock logic lives in the seam.
unsafe extern "Rust" {
    fn __dysnomia_mono_ns() -> i64;
    fn __dysnomia_wall_ns() -> i64;
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Instant(Duration);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct SystemTime(Duration);

pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::from_secs(0));

impl Instant {
    pub fn now() -> Instant {
        // `__dysnomia_mono_ns` is monotone and non-negative by construction (urt's
        // zero-base `utc_ns_at`); `max(0)` re-establishes `from_nanos`'s `u64` domain.
        // SAFETY: a pure delegation to the seam — reads two registers, no memory effects.
        let ns = unsafe { __dysnomia_mono_ns() };
        Instant(Duration::from_nanos(ns.max(0) as u64))
    }

    pub fn checked_sub_instant(&self, other: &Instant) -> Option<Duration> {
        self.0.checked_sub(other.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_sub(*other)?))
    }
}

impl SystemTime {
    pub const MAX: SystemTime = SystemTime(Duration::MAX);

    pub const MIN: SystemTime = SystemTime(Duration::ZERO);

    pub fn now() -> SystemTime {
        // The MVP RTC is always post-1970, so the wall clock is non-negative; `max(0)`
        // re-establishes `from_nanos`'s `u64` domain for any pre-epoch corruption.
        // SAFETY: a pure delegation to the seam.
        let ns = unsafe { __dysnomia_wall_ns() };
        SystemTime(Duration::from_nanos(ns.max(0) as u64))
    }

    pub fn sub_time(&self, other: &SystemTime) -> Result<Duration, Duration> {
        self.0.checked_sub(other.0).ok_or_else(|| other.0 - self.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_sub(*other)?))
    }
}
