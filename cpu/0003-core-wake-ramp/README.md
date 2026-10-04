# 0003 — A core that sleeps more than ~1–2 ms restarts ~20% slower for up to 1 ms

**Status:** single machine
**Area:** cpu
**Date:** 2026-10-04

## Question

In a thread pool, workers that finish early wait for the others; if they
block (sleep) instead of spinning, their core goes idle. **How much clock
does a core lose when its thread wakes up, for how long, and after how much
sleep?** And is it per core, or does the whole chip slow down?

## Machine

| Item | Value |
|---|---|
| CPU | AMD Ryzen 7 5700X (Zen 3), 8 cores / 16 threads, 32 MB L3 |
| OS | Windows, Memory Integrity on; power plan not recorded for this run (Balanced is the default) |
| Privileges | Normal user, no admin, no driver |
| Toolchain | Rust, MSVC, release build |
| Tool | Ember (CPU lab, private for now): `ember loadstep` |

## Protocol

One worker thread per logical CPU (16), each pinned to its CPU. Each trial:

1. **Lead-in:** 30 ms of AVX2 FMA work on every worker (busy chip).
2. **End of the phase**, one of:
   - `uneven`: workers finish one by one, spread over 9–17 ms (order
     shuffled per trial), so they wait at the barrier for 0–8 ms;
   - `even`: all finish together;
   - `rest`: 300 ms asleep;
   - `steady`: no change (baseline).
3. **Blocking barrier** (`std::sync::Barrier`): early finishers sleep.
4. **Next load** for 25 ms, `heavy` (FMA) or `light` (scalar adds),
   interleaved with ~14 µs sensor windows that measure the effective clock
   twice: a timed chain of dependent adds, and APERF/MPERF via `RDPRU`
   (see [finding 0002](../0002-rdpru-user-mode-windows/)).

Variants: light pre-spin of 0.3–10 ms before the next load, and one thread
per physical core. 14 configurations × 30 trials in randomized blocks,
150 ms between trials, one session of ~2 minutes. See
[PROTOCOL.md](../../PROTOCOL.md); the CPU was idle for ~2 minutes before
the session.

## Results

Full report: [`data/loadstep-report.txt`](data/loadstep-report.txt).
Per-thread summary: [`data/idle-vs-first-bin.csv`](data/idle-vs-first-bin.csv).

### The dip, by configuration

Median clock (GHz) by time since the next load started:

| Configuration | 0–0.5 ms | 0.5–1 ms | 1–2 ms | 15–25 ms |
|---|---|---|---|---|
| uneven → heavy | **3.70** | 4.57 | 4.62 | 4.53 |
| uneven → light | **3.71** | 4.62 | 4.63 | 4.63 |
| rest → heavy | **3.58** | **3.62** | 4.63 | 4.55 |
| rest → light | **3.58** | **4.09** | 4.63 | 4.63 |
| even → heavy | 4.53 | 4.53 | 4.53 | 4.52 |
| steady → heavy | 4.53 | 4.53 | 4.52 | 4.50 |
| uneven → heavy, pre-spin 0.3 ms | **3.70** | 4.62 | 4.63 | 4.53 |
| uneven → heavy, pre-spin 1–10 ms | 4.62–4.63 | 4.62–4.63 | 4.61 | 4.53 |
| uneven → heavy, 1 thread/core | **3.70** | **3.71** | 4.63 | 4.63 |

APERF/MPERF give the same picture within ~0.3% (e.g. 3.71 GHz in the first
bin of uneven → heavy), so this is a real clock drop, not an
instruction-issue limit.

### It is per core, with a threshold

First-bin clock of each thread vs how long **that thread** slept at the
barrier (uneven → heavy, 480 thread-trials):

| Thread slept | n | Clock, first 0.5 ms (median) |
|---|---|---|
| < 0.05 ms (last to finish) | 29 | 4.625 GHz |
| 0.3–1 ms | 30 | 4.628 GHz |
| 1–2 ms | 60 | 4.560 GHz (mixed) |
| 2–4 ms | 119 | **3.694 GHz** |
| 4–6 ms | 117 | **3.694 GHz** |
| 6–8 ms | 91 | **3.694 GHz** |
| ≥ 8 ms | 29 | **3.694 GHz** |

The same pattern holds for the light next load and with one thread per
core. In every trial, the threads that did not sleep start at full clock
while their neighbours start at 3.7 GHz: 2 to 12 of the 16 threads dip per
trial, never all.

## What was ruled out

| Hypothesis | Test | Verdict |
|---|---|---|
| A chip-wide slowdown when cores wake up | Same trial: sleepers vs non-sleepers | No: only cores that slept dip |
| The load step itself (current draw of the new load) | Heavy vs light next load | No: both dip the same |
| Sleep length scales the dip | Per-thread sleep 2 ms vs 8 ms | No: a step, the same 3.70 GHz; deeper (3.58) only after 300 ms |
| Short spin keeps the clock up | Pre-spin 0.3 ms vs ≥ 1 ms | 0.3 ms does not help; ≥ 1 ms removes the dip |

## Conclusion

**Supported by the data:** on this Zen 3 under Windows, a core whose thread
sleeps for more than roughly **1–2 ms** restarts at a lower clock
(**~3.70 GHz** instead of 4.63, about **−20%**; **~3.58 GHz**, −23%, after
a long idle) and needs **0.5–1 ms** to get back. Shorter waits cost
nothing. The effect is per core, independent of what the core runs next,
and disappears if the thread spins for at least ~1 ms instead of sleeping.

**Hypothesis:** the discrete levels (3.58 / 3.70 / ~4.1 GHz) suggest the
idle core is parked at a lower performance state and ramped back by the
firmware's ~1 ms control loop.

**Practical rule for thread pools:** a worker that would wait more than
~1 ms should spin (or spin ≥ 1 ms before blocking) if the next phase is
latency-sensitive; otherwise its first ~0.5 ms run ~20% slower.

**What this does not explain:** [finding 0001](../0001-post-attention-slowdown/)
(1.5–1.8× slower on *all* threads, including those that never idled, for
~10 ms) is a different, larger effect.
[Finding 0004](../0004-post-streaming-clock-depression/) covers that larger
effect: a chip-wide clock reduction after fast streaming reads.

## Open questions

- Does the 1–2 ms threshold depend on the power plan (not recorded for
  this run)? On Zen 2/4/5, on Intel?
- Why does recovery take longer (~1 ms vs ~0.5 ms) with one thread per
  core? A guess: the whole core, both SMT siblings, was idle.

## Replications

| Who | CPU | Result | Link |
|---|---|---|---|
| — | — | — | — |
