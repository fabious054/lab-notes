# 0002 — APERF/MPERF are readable from user mode on Windows through RDPRU

**Status:** single machine (tested with Memory Integrity off and on)
**Area:** cpu
**Date:** 2026-10-04

## Question

AMD Zen 2 and later have `RDPRU`, an instruction that reads two
model-specific registers from user mode: MPERF (reference cycles, at the
TSC rate on Zen) and APERF (actual core cycles). Their ratio over an
interval is the true average core clock. On Linux it is known to work for
normal users. **Does Windows leave it usable from user mode, with no admin
rights and no driver?** We found no source that answers this.

## Machine

| Item | Value |
|---|---|
| CPU | AMD Ryzen 7 5700X (Zen 3), 8 cores / 16 threads |
| OS | Windows (exact build not recorded); tested with Core isolation → Memory Integrity **off**, then **on** (msinfo32: "Hypervisor enforced Code Integrity" running) |
| Privileges | Normal user, no admin, no driver |
| Toolchain | Rust, MSVC, release build |
| Tool | Ember (CPU lab, private for now): `ember __rdpru-sample` and `ember sensor` |

## Protocol

1. Check CPUID Fn8000_0008 EBX bit 4 (RDPRU advertised).
2. In a **separate child process**, so that a fault cannot crash the main
   program: pin the thread to the CPU it starts on, spin for 20 ms, read
   TSC, MPERF (`RDPRU` with ECX=0) and APERF (ECX=1); repeat 6 times.
3. Accept the counters only if, between samples, every counter increases,
   ΔMPERF/ΔTSC is within 10% of 1, ΔAPERF/ΔMPERF is between 0.2 and 3.0,
   and APERF is not a copy of MPERF.
4. Cross-check: for each ~14 µs window of Ember's sensor, compare the
   clock from ΔAPERF/ΔMPERF × TSC rate with the clock inferred from a
   timed chain of dependent integer adds (1 cycle each).

The code to read the counters is in [`rdpru.rs`](rdpru.rs).

## Results

### The instruction runs, and the counters are coherent

Raw child output ([`data/child-pinned.txt`](data/child-pinned.txt)),
columns TSC, MPERF, APERF:

```
# pinned to cpu 8
652797504210253 22814580962760 28166737410315
652797572398919 22814649129836 28166830636775
652797640498403 22814717229287 28166923766896
652797708608359 22814785339276 28167016912520
652797776750921 22814853481838 28167110105555
652797844876143 22814921607060 28167203272879
```

| Interval | ΔTSC | ΔMPERF | ΔMPERF/ΔTSC | ΔAPERF/ΔMPERF | Clock |
|---|---|---|---|---|---|
| 1 | 68,188,666 | 68,167,076 | 0.9997 | 1.3676 | 4.642 GHz |
| 2 | 68,099,484 | 68,099,451 | 1.0000 | 1.3676 | 4.642 GHz |
| 3 | 68,109,956 | 68,109,989 | 1.0000 | 1.3676 | 4.642 GHz |
| 4 | 68,142,562 | 68,142,562 | 1.0000 | 1.3676 | 4.642 GHz |
| 5 | 68,125,222 | 68,125,222 | 1.0000 | 1.3676 | 4.642 GHz |

TSC rate 3.394 GHz. MPERF tracks the TSC exactly while the core spins, as
expected on Zen, and APERF runs 1.3676× faster: 4.642 GHz, the 5700X's
single-core boost.

### Same result with Memory Integrity on (Windows on the Hyper-V layer)

After enabling Memory Integrity and rebooting (msinfo32 lists
"Hypervisor enforced Code Integrity" among the running virtualization-based
security services), the same check
([`data/child-pinned-hvci.txt`](data/child-pinned-hvci.txt)):

| Interval | ΔTSC | ΔMPERF | ΔMPERF/ΔTSC | ΔAPERF/ΔMPERF | Clock |
|---|---|---|---|---|---|
| 1 | 68,112,132 | 68,112,132 | 1.0000 | 1.3676 | 4.642 GHz |
| 2 | 68,070,040 | 68,067,252 | 1.0000 | 1.3676 | 4.642 GHz |
| 3 | 68,262,174 | 68,247,826 | 0.9998 | 1.3676 | 4.642 GHz |
| 4 | 68,220,966 | 68,220,966 | 1.0000 | 1.3676 | 4.642 GHz |
| 5 | 68,286,960 | 68,286,927 | 1.0000 | 1.3676 | 4.642 GHz |

The hypervisor neither blocks nor virtualizes the counters: same ratio,
same clock, to four digits.

### It agrees with an independent clock measurement

Full output: [`data/sensor.txt`](data/sensor.txt).

| Pass | Clock from timed instructions | Clock from APERF/MPERF | Gap |
|---|---|---|---|
| One thread at a time, each physical core | 4.628 GHz ± 0.1% | 4.643 GHz ± 0.1% | 0.3% |
| 8 threads, one per core | 4.628 GHz | 4.642–4.643 GHz | 0.3% |
| 16 threads (SMT) | 4.554 GHz | 4.567–4.568 GHz | 0.3% |

With Memory Integrity on ([`data/sensor-hvci.txt`](data/sensor-hvci.txt)):
4.628 vs 4.642–4.643 GHz (passes 1–2) and 4.526 vs 4.543 GHz (pass 3),
the same 0.3–0.4% gap.

The constant 0.3% offset is consistent with the timing overhead
(`lfence; rdtsc`, ~100 cycles) included in the instruction-timed load of
36,864 cycles.

### Pitfall: pin the thread

The first attempt did not pin the child process and failed validation
(`NotIncreasing`, [`data/child-unpinned.txt`](data/child-unpinned.txt)).
APERF and MPERF are **per-core** counters: if the OS moves the thread to
another core between two reads, the second read comes from a different
counter and can go backwards. Any user-mode use of `RDPRU` must keep the
thread on one core for the whole interval.

## Conclusion

**Supported by the data:** on this Windows machine, with Memory Integrity
both off and on, `RDPRU` runs from user mode with no admin rights and no driver, and APERF/MPERF are real (not
zero, not virtualized copies). The true core clock they give agrees with an
independent instruction-timing measurement within 0.3%.

This gives a user-mode program on Windows a direct, cheap reading of the
real core clock on Zen 2+, without the admin rights or kernel driver that
reading these registers normally requires on Windows.

## Open questions

- Which Windows builds? Zen 2 and Zen 4/5 parts?
- Intel CPUs do not implement `RDPRU`; there is no equivalent user-mode
  path there.

## Replications

| Who | CPU | Result | Link |
|---|---|---|---|
| — | — | — | — |
