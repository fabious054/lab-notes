//! Reading APERF/MPERF from user mode with RDPRU (AMD Zen 2+), as used in
//! finding 0002. Excerpt; Ember wraps this in a child-process check.
//!
//! WARNING: if the CPU lacks RDPRU or the OS disables it, executing it
//! faults. Check CPUID first, and try it in a process that may crash.
//! Pin the thread to one core: the counters are per core.

/// CPUID Fn8000_0008 EBX bit 4.
pub fn rdpru_advertised() -> bool {
    use core::arch::x86_64::__cpuid;
    #[allow(unused_unsafe)]
    unsafe {
        __cpuid(0x8000_0000).eax >= 0x8000_0008 && __cpuid(0x8000_0008).ebx & (1 << 4) != 0
    }
}

/// 0 = MPERF, 1 = APERF.
///
/// # Safety
/// Faults if RDPRU is unavailable (see above).
pub unsafe fn rdpru(id: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    core::arch::asm!(
        ".byte 0x0f, 0x01, 0xfd", // rdpru
        in("ecx") id,
        out("eax") lo,
        out("edx") hi,
        options(nomem, nostack),
    );
    ((hi as u64) << 32) | lo as u64
}

/// Average core clock in Hz over an interval, from two (MPERF, APERF)
/// readings on the same core and the TSC rate (MPERF ticks at the TSC rate
/// on Zen).
pub fn clock_hz(m0: u64, a0: u64, m1: u64, a1: u64, tsc_hz: f64) -> f64 {
    (a1 - a0) as f64 / (m1 - m0) as f64 * tsc_hz
}
