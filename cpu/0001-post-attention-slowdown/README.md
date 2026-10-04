# 0001 — Cores run 1.5–1.8× slower for ~10 ms after an unevenly ending parallel phase

**Status:** single machine — mechanism confirmed inside the engine (2026-10-04, see [below](#confirmation-inside-the-engine-2026-10-04)); explained by [finding 0004](../0004-post-streaming-clock-depression/)
**Area:** cpu
**Date:** 2026-10-03

## Question

In my CPU LLM engine, one matrix multiplication of the Qwen3-1.7B model
(`o_proj`, 2048×2048) took about twice as long inside the forward pass
as in isolation, while another one with the **same shape** (`q_proj`)
did not. Why?

## Machine

| Item | Value |
|---|---|
| CPU | AMD Ryzen 7 5700X (Zen 3), 8 cores / 16 threads, AVX2 + FMA, 512 KB L2 per core, 32 MB L3 |
| Memory | DDR4-2400, dual channel |
| OS / power plan | Windows; Balanced (default), plus one run on High performance |
| Toolchain / build | Rust, MSVC, `cargo test --release` (optimized) |
| Threads | 16-thread pool for the in-forward runs (12 in one isolated run, see `data/v1.txt`) |
| Workload | Prefill of 512 synthetic tokens, Qwen3-1.7B Q8_0, quantized matrix multiplications |

## Protocol

Seven diagnostic builds, each testing one hypothesis, run as ignored
tests (`cargo test --release <name> -- --ignored --nocapture`) after a
warm-up pass. Times are summed over the model's 28 layers unless marked
per layer. Raw output of every round: [`data/`](data/) (`v1.txt` …
`v7.txt`).

Deviation from [PROTOCOL.md](../../PROTOCOL.md): these were diagnostic
runs, not benchmarks, so they were not paused or alternated. Each round
compares positions **inside the same run**, which cancels most drift.

## Results

### The slowdown is about *where* the work runs, not *what* it is

| Measurement | Result | Raw |
|---|---|---|
| `q_proj` vs `o_proj` in isolation, same input | 8.1 vs 8.1 ms per layer | v1 |
| Inside the forward pass | `q_proj` ~6.5 ms, `o_proj` ~14.6 ms per layer | v2 |
| `q_proj` moved to run right after attention | 370 ms (vs 181 ms in its own place); the `o_proj` after it drops to 258 ms | v6 |

Any matrix multiplication placed right after the attention phase runs
about 2× slower, and the effect fades after roughly 10 ms of work.

### The cores themselves are slower there — even with no memory access

Every thread of the pool ran two probes that touch no memory: a
dependent integer chain (200,000 multiply-adds, tracks the core clock)
and 8 independent AVX2 FMA chains (50,000 iterations, tracks vector
throughput). Medians over all layers and threads:

| Point | Integer chain | AVX2 FMA |
|---|---|---|
| Before `q_proj` | 24.2 µs | 92.8 µs |
| Right after attention | **43.4 µs (1.8×)** | **138.8 µs (1.5×)** |
| After `o_proj` (~10 ms later) | 35.9 µs | 108.0 µs |

Raw: `data/v7.txt`. Probe code: [`probe.rs`](probe.rs).

### What the attention phase looks like

Attention ran one task per head: 16 heads on 16 threads, identical work
per head. The heads finished between ~9 and ~17 ms, so cores went idle
one by one; only 73–88% of the pool's time was used. Right after, all 16
threads start the next multiplication together (`data/v4.txt`,
`data/v6.txt`).

## What was ruled out

| Hypothesis | Test | Verdict |
|---|---|---|
| The weights (scale distribution, subnormal values) | Isolated, same input | No — same time as `q_proj` (v1) |
| The input data coming from attention | 2×2 of {weights} × {real inputs}, isolated | No — all ≈ 8 ms; no subnormal or non-finite values (v2) |
| The input conversion step | Timed separately inside the forward | No — the extra cost is in the multiplication itself (v3) |
| Cache layout of attention's output (written by 16 cores) | Multiply a fresh single-thread copy instead | No — it got slower, not faster (v3) |
| Output alignment / false sharing | Output buffer at +0/16/32/48 bytes | No — no difference (v5) |
| Windows power plan / core parking | Balanced vs High performance | No — 415 vs 417 ms (v4) |
| Threads joining late | Per-task timeline | No — all 16 threads start at 0 ms; **every task** is slower: 1.3–1.5 ms vs 0.72 ms (v6) |
| Threads asleep before the multiplication | 0.3 ms busy-spin on every thread first | No — still 361 ms (v6) |

## Conclusion

**Supported by the data:** for roughly 10 ms after an attention phase that
ends unevenly, every core of this Zen 3 runs about 1.5–1.8× slower on
any work, including arithmetic that never touches memory. It happens
below the operating system: the power plan made no difference.

**Hypothesis (not yet measured):** the chip's own power management reacts
to the load pattern. Two candidate mechanisms:

- **Clock ramp after idle:** cores that went idle in the uneven tail of
  attention lose clock and take milliseconds to ramp back.
- **Current limit on a load step:** all cores jump to heavy AVX2 work at
  once, and the chip holds performance down until current settles.

*Update 2026-10-04:* synthetic tests ruled out both as the main cause and
point to a third mechanism: a chip-wide clock reduction during fast
streaming reads, which the next phase inherits for ~10 ms
([finding 0004](../0004-post-streaming-clock-depression/); details below).
A clock sensor inside the engine then confirmed it: the effective clock
of every pool thread falls right after attention, as low as 0.54–0.59
GHz on Qwen3-1.7B, and is back by the end of the layer
([confirmation](#confirmation-inside-the-engine-2026-10-04)).

I did not find this documented for Zen 3. The closest documented case
is on Zen 5, where heavy vector load limits throughput for a similar
time scale.

## Open questions

- ~~Which mechanism?~~ Answered: a chip-wide clock drop after fast
  streaming reads (finding 0004), measured inside the engine around
  attention (section below).
- ~~Real clock drop or an instructions-per-cycle limit?~~ A clock drop:
  the in-engine sensor is a dependent-add chain timed with the TSC, which
  instruction-per-cycle limits on memory cannot slow, and in the
  synthetic tests it agreed with APERF/MPERF within ~0.3%.
- Why is Qwen3-1.7B hit so much harder (0.14× of the pre-attention clock)
  than 0.6B (0.86×) and 4B (0.67×)? Not explained yet. 0.6B and 1.7B have
  the same attention shape (16 query heads, 8 K/V heads, head_dim 128),
  but in these runs 0.6B used 16 threads and 1.7B used 12. A rerun of
  1.7B on 16 threads is the first thing to check.
- Does it happen on other Zen 3 chips, other generations, Intel?
- Balancing the attention work (finer tasks, shorter idle tail) cut the
  `o_proj` time by 19% on the smaller Qwen3-0.6B, but left it unchanged
  on the 1.7B and 4B. Why?

## Synthetic reproduction attempt (2026-10-04, part 2)

Ember's `ember loadstep` recreated the pattern without the LLM engine: 16
pinned threads, a 30 ms busy lead-in, a phase whose threads finish one by
one over 9–17 ms (like the attention heads), a blocking barrier, then a new
load, with the clock read every ~64 µs by timed instructions and by
APERF/MPERF.

- It **did not reproduce** the 1.5–1.8×, ~10 ms slowdown on all threads.
- It found a smaller, separate effect: cores whose thread **slept** more
  than ~1–2 ms restart ~20% slower for 0.5–1 ms, while threads that did
  not sleep start at full clock in the same trial
  ([finding 0003](../0003-core-wake-ramp/)).
- Limitation: the synthetic "heavy" load (FMA on constant registers) draws
  little power (16 threads lose only ~2% clock in steady state), so the
  current-limit hypothesis was **not tested** by this run. The next
  section covers the follow-up with high-power and memory-streaming loads.

## Synthetic tests of the mechanism (2026-10-04, parts 3–5)

Ember's `ember loadstep --set power`, `--set stream` and `--set trigger`
replaced the constant-register load with loads on random data (int8 dot
products, DRAM and L3 streaming, pointer chasing) and sampled the clock
during the previous phase too. Full write-up:
[finding 0004](../0004-post-streaming-clock-depression/).

- **Current limit (H2): not supported.** A load step into high-power int8
  work starts fast (4.62 GHz) and settles to the ~4.05 GHz power limit
  over ~10 ms; after a power-limited phase the clock is back at once.
- **Streaming reads: reproduces the shape.** During fast streaming reads
  the whole chip lowers its clock (to ~3.7–4.3 GHz from DRAM, ~2.3 GHz from
  L3), and the next phase starts there and needs 10–15 ms, on every core.
  Worst trials reach 2.5–3.0 GHz against 4.6, the 1.5–1.8× of this
  finding; with L3-sized private slices, down to 0.5 GHz for 40+ ms.
- **Memory stalls without bandwidth (pointer chasing): no effect.**

So the leading explanation is now: the attention phase (its K/V likely
fits in L3 at 512 tokens) streams fast enough to lower the chip's clock,
and `o_proj` pays for it during the ~10 ms the clock needs to recover.
Measured inside the engine on 2026-10-04: see the next section.

## Confirmation inside the engine (2026-10-04)

CandleCLI's `/bench 512 0 --clock` (issue #120) runs a ~8 µs clock sensor
on **every thread of the pool** at five points of every layer during a
512-token prefill: one dependent integer add per cycle, timed with the
TSC, no memory access (the same method as Ember's sensor). Fast (int8)
mode, Ryzen 7 5700X, 3 repetitions, all layers and threads pooled.
Raw: [`data/v8-clock.txt`](data/v8-clock.txt).

Median effective clock in GHz (10th percentile in brackets):

| Point in each layer | Qwen3-0.6B (16 threads) | Qwen3-1.7B (12) | Qwen3-4B (12) |
|---|---|---|---|
| Before Q/K/V | 4.15 (3.93) | 4.20 (3.48) | 4.28 (3.51) |
| Before attention | 4.05 (3.94) | 4.13 (4.02) | 4.12 (4.01) |
| **Right after attention** | **3.58 (3.16)** | **0.59 (0.54)** | **2.88 (2.72)** |
| After `o_proj` | 3.52 (2.94) | 2.48 (2.36) | 4.13 (3.96) |
| After the MLP | 4.08 (3.92) | 4.04 (3.91) | 4.08 (3.36) |
| After attention ÷ before Q/K/V | 0.862 | **0.140** | 0.673 |

- **The clock falls during attention and the next multiplication starts
  there.** Before attention every model runs at ~4.1 GHz; right after,
  every thread reads lower. By the end of the MLP all three are back at
  ~4.05 GHz.
- **Qwen3-1.7B is the extreme case:** `o_proj` starts at 0.54–0.59 GHz and
  still runs at only ~2.5 GHz when it ends. 0.54 GHz is the same floor
  Ember measured with L3-sized private slices (finding 0004). This
  matches the original question: `o_proj` took 528 ms per pp512 against
  ~210 ms expected from `q_proj` (same 2048×2048 shape; `q_proj` is about
  half of the Q/K/V stage's 419 ms) — ~300 ms, ~9% of the prefill.
- **Qwen3-4B** drops less and recovers before `o_proj` ends, and its
  `o_proj` shows almost no excess (665 ms against ~640 ms expected).
- The sensor took 0.13–0.84% of the prefill; with it off, outputs were
  identical to the build without it.

Caveat: each sample is a `rayon::broadcast`, so it wakes every worker and
is itself a synchronization point. It reads the clock a worker has when
the next phase would start, which is what that phase pays; it does not
read the clock of a core that stays asleep.

## Replications

| Who | CPU | Result | Link |
|---|---|---|---|
| — | — | — | — |
