# 0001 — Cores run 1.5–1.8× slower for ~10 ms after an unevenly ending parallel phase

**Status:** draft — single machine; mechanism lead in [finding 0004](../0004-post-streaming-clock-depression/), not yet confirmed in the engine
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

I did not find this documented for Zen 3. The closest documented case
is on Zen 5, where heavy vector load limits throughput for a similar
time scale.

## Open questions

- **Which mechanism?** Synthetic tests (below) ruled out the per-core
  idle ramp as the main cause (finding 0003: ~20% for under 1 ms, sleepers
  only) and the current-limit hypothesis (a power-limited phase releases
  at once). They found a chip-wide slowdown after fast streaming reads,
  strongest from L3 (finding 0004), with this finding's shape and size in
  the worst trials. Still to confirm: that it happens around attention
  inside the engine.
- Is it a real clock drop or an instructions-per-cycle limit? These
  probes cannot tell the two apart. (In the synthetic tests it is a real
  clock drop: APERF/MPERF agree.)
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
and `o_proj` pays for it during the ~10 ms the clock needs to recover. Not
yet measured inside the engine.

## Replications

| Who | CPU | Result | Link |
|---|---|---|---|
| — | — | — | — |
