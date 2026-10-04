//! Memory-free CPU probes used in finding 0001.
//!
//! Excerpt from the diagnostic build: each thread of a rayon pool runs
//! both probes at the same moment, and the caller records the times at
//! three points of the workload (before a phase, right after attention,
//! ~10 ms later). Neither probe touches memory, so a slowdown means the
//! core itself is slower, not the cache or RAM.
//!
//! A standalone runner will ship with Ember (the CPU lab). Until then,
//! these functions can be dropped into any Rust program that depends on
//! `rayon`.

/// One probe per pool thread, all at once.
/// Returns (integer chain µs, AVX2 FMA µs) per thread.
pub fn probe_all() -> Vec<(f64, f64)> {
    use rayon::prelude::*;
    let n = rayon::current_num_threads();
    (0..n).into_par_iter().map(|_| probe_one()).collect()
}

fn probe_one() -> (f64, f64) {
    // Dependent integer chain: each step needs the previous result, so the
    // time tracks the core clock.
    let t = std::time::Instant::now();
    let mut x: u64 = std::hint::black_box(1);
    for i in 0..200_000u64 {
        x = x.wrapping_mul(0x9E37_79B9).wrapping_add(i);
    }
    std::hint::black_box(x);
    let dep = t.elapsed().as_secs_f64() * 1e6;
    let fma = fma_probe();
    (dep, fma)
}

#[cfg(target_arch = "x86_64")]
fn fma_probe() -> f64 {
    if !(is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma")) {
        return 0.0;
    }
    // SAFETY: features checked just above.
    unsafe { fma_probe_avx2() }
}

#[cfg(not(target_arch = "x86_64"))]
fn fma_probe() -> f64 {
    0.0
}

/// 8 independent AVX2 FMA chains: enough to keep the FMA units busy, so
/// the time tracks vector throughput.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
unsafe fn fma_probe_avx2() -> f64 {
    use std::arch::x86_64::*;
    let a = _mm256_set1_ps(0.999_999);
    let b = _mm256_set1_ps(1e-7);
    let mut acc = [_mm256_set1_ps(1.0); 8];
    let t = std::time::Instant::now();
    for _ in 0..50_000 {
        for v in acc.iter_mut() {
            *v = _mm256_fmadd_ps(*v, a, b);
        }
    }
    let us = t.elapsed().as_secs_f64() * 1e6;
    std::hint::black_box(acc);
    us
}
