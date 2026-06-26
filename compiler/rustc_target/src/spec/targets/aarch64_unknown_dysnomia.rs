//! Dysnomia OS userspace (EL0), AArch64 softfloat.
//!
//! The ABI is deliberately softfloat with SIMD codegen disabled: the kernel
//! saves and restores general-purpose registers only on trap, so hardware
//! FP/NEON state in EL0 would be silently corrupted across preemption. The
//! Applications provide `_start` and the 38 `__dysnomia_pal_v1_*` symbols
//! documented by Dysnomia's standalone PAL ABI checker. Links are fully static
//! through `rust-lld`, without C runtime objects.

use crate::spec::{
    Arch, Cc, CfgAbi, LinkerFlavor, Lld, Os, PanicStrategy, RelocModel, RustcAbi, StackProbeType,
    Target, TargetMetadata, TargetOptions,
};

pub(crate) fn target() -> Target {
    let opts = TargetOptions {
        os: Os::Dysnomia,
        cfg_abi: CfgAbi::SoftFloat,
        rustc_abi: Some(RustcAbi::Softfloat),
        linker_flavor: LinkerFlavor::Gnu(Cc::No, Lld::Yes),
        linker: Some("rust-lld".into()),
        features: "+v8a,+strict-align,-neon".into(),
        relocation_model: RelocModel::Static,
        disable_redzone: true,
        max_atomic_width: Some(128),
        stack_probes: StackProbeType::Inline,
        panic_strategy: PanicStrategy::Abort,
        default_uwtable: true,
        ..Default::default()
    };
    Target {
        llvm_target: "aarch64-unknown-none".into(),
        metadata: TargetMetadata {
            description: Some("Dysnomia OS userspace (EL0), AArch64 softfloat".into()),
            // Not an upstream-supported target; no tier is claimed.
            tier: None,
            host_tools: Some(false),
            std: Some(true),
        },
        pointer_width: 64,
        data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128-Fn32".into(),
        arch: Arch::AArch64,
        options: opts,
    }
}
