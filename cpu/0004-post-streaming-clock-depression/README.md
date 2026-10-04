# 0004 — Fast streaming reads lower the whole chip's clock, and the next phase pays for 10–40 ms

**Status:** single machine
**Area:** cpu
**Date:** 2026-10-04

## Question

[Finding 0001](../0001-post-attention-slowdown/) saw every core of a Zen 3
run 1.5–1.8× slower for ~10 ms after the attention phase of an LLM
engine. [Finding 0003](../0003-core-wake-ramp/) explained only a small,
per-core part of it. **Does the kind of work in the previous phase change
the clock of the next one, on every core, for milliseconds?** And if so,
which part of that work triggers it?

## Machine

| Item | Value |
|---|---|
| CPU | AMD Ryzen 7 5700X (Zen 3), 8 cores / 16 threads, 512 KB L2 per core, 32 MB L3 |
| Memory | DDR4-2400, dual channel |
| OS | Windows, Memory Integrity on; power plan not recorded (Balanced is the default) |
| Privileges | Normal user, no admin, no driver |
| Toolchain | Rust, MSVC, release build |
| Tool | Ember (CPU lab, private for now): `ember loadstep --set power`, `--set stream`, `--set trigger` |

## Protocol

One worker thread per logical CPU, each pinned to its CPU. Each trial:

1. **Lead-in**, 30 ms (100 ms in one arm), then the **end of the
   previous phase**: `uneven` (threads finish one by one over 9–17 ms, order
   shuffled per trial), `even` (all at 13 ms) or `steady` (no change,
   baseline).
2. **Blocking barrier** (`std::sync::Barrier`): early finishers sleep.
3. **Next load** for 25 or 50 ms, interleaved with ~14 µs sensor windows
   that measure the effective clock twice: a timed chain of dependent adds,
   and APERF/MPERF via `RDPRU` (see
   [finding 0002](../0002-rdpru-user-mode-windows/)).

In `--set stream` and `--set trigger`, the sensor also runs **during the
previous phase** (one window every 250 µs). There the clock is
instruction-timed only; after the barrier both methods agree within ~0.3%.

Loads (all on random data, so bits actually toggle):

| Load | What it does |
|---|---|
| `light` | scalar adds; its steady clock is the chip's maximum (4.60–4.63 GHz) |
| `int8` | AVX2 int8 dot products (`vpmaddubsw`/`vpmaddwd`) from L1, high power |
| `stream` | reads one shared 96 MB buffer (3× L3), each thread from its own offset |
| `stream-slices` | each thread reads only its own 6 MB slice of that buffer |
| `stream-l3` | reads a 16 MB buffer (fits in L3) |
| `chase` | pointer chasing through 64 MB: memory stalls, little bandwidth |
| `stream-rw` | DRAM reads plus stores to a private buffer per thread |

13, 14 and 12 configurations × 30 trials in randomized blocks, 150 ms
between trials, one session of ~2 minutes per set, the CPU idle for ~2
minutes before each (see [PROTOCOL.md](../../PROTOCOL.md)). Every
configuration is compared with the `steady` arm that runs the same next
load on the same threads.

## Results

Full reports: [`data/power-report.txt`](data/power-report.txt),
[`data/stream-report.txt`](data/stream-report.txt),
[`data/trigger-report.txt`](data/trigger-report.txt). Per configuration:
[`data/config-summary.csv`](data/config-summary.csv) (the spread of the
2–10 ms clock across trials, and how well the clock at the end of the
previous phase predicts it).

### The clock falls while streaming, and the next phase starts there

`--set stream`, all threads, previous phase `stream`, next load `light`
(clock in GHz, median):

| Previous phase, ms from end of lead-in | −20..−10 | −10..0 | 0..4 | own last 2 ms |
|---|---|---|---|---|
| | 4.47 | 4.01 | 3.76 | 3.81 |

| Next load, ms since it started | 0–0.5 | 1–2 | 3–5 | 5–10 | 10–15 | 15–50 |
|---|---|---|---|---|---|---|
| after `stream uneven` | 3.40 | 3.74 | 3.93 | 4.23 | 4.59 | 4.60 |
| `light steady` (baseline) | 4.60 | 4.60 | 4.60 | 4.60 | 4.60 | 4.60 |

- **It is chip-wide.** In each trial every core runs the same clock over
  2–10 ms (spread ~0.01 GHz), including threads that never slept at the
  barrier.
- **It carries over.** Per trial, the clock at the end of the previous
  phase predicts the 2–10 ms clock afterwards (Pearson r 0.81 and 0.80 in
  the `even` arms, 0.48–0.68 in the `uneven` ones). Trials whose stream
  ended near 4.6 GHz show no slowdown at all.
- **It varies a lot from trial to trial**, so medians understate it. All
  threads, uneven end: 17 of 30 trials below 90% of the baseline over
  2–10 ms, the worst at 2.95 GHz. An even end shows it too (11/30, worst
  2.84 GHz): an idle tail is not required.

### A power-limited phase releases at once; streaming does not

| Previous phase (one thread per core) | Clock at its end | Next load 2–10 ms vs steady | Slow trials |
|---|---|---|---|
| `int8` (power-limited) | 4.21 | 1.000 | 0/30 |
| `stream` | 4.08 | 0.926 | 13/30 |

`int8` holds the chip at its power limit (4.06–4.10 GHz while it runs),
lower than many streaming phases end at, yet the next phase runs at
4.63 GHz at once on every thread that did not sleep. Whatever lowers the
clock during streaming releases over ~10 ms: two separate mechanisms.

### What triggers it: fast streaming reads, most of all from L3

`--set trigger`, all threads, next load `light`:

| Previous phase | Clock while running (−20..−10 ms) | 2–10 ms vs steady | Slow trials | Worst trial |
|---|---|---|---|---|
| `stream` (DRAM) | 4.55 | 0.958 | 6/30 | 3.29 |
| `stream-l3` (fits L3) | **2.26** | **0.817** | **29/30** | 3.48 |
| `chase` (stalls, little bandwidth) | 4.61 | 1.000 | 0/30 | 4.63 |
| `stream-rw` (DRAM read + write) | 4.59 | 1.000 | 0/30 | 4.63 |

- Streaming from **L3** drops the clock to ~2.3 GHz within ~10 ms; the
  next phase starts at ~3.4 GHz and needs 10–15 ms.
- **Pointer chasing does nothing:** memory stalls by themselves do not
  lower the clock.
- `stream-rw` moves far fewer bytes per second than `stream` (one load and
  one store per vector, not unrolled), so its null result says "less read
  bandwidth, no effect", not that writes prevent it.
- With longer DRAM streaming (100 ms), the clock settles near **3.7 GHz**
  after 40–60 ms; afterwards 18/30 slow trials, worst 2.46 GHz.

### The worst case: private slices, 0.5–1.9 GHz for 40+ ms

`stream-slices` with one thread per core (8 × 6 MB = 48 MB, more than the
32 MB L3), next load `light`:

| | 0–0.5 | 2–3 | 5–10 | 15–25 | 25–40 | 40–50 ms |
|---|---|---|---|---|---|---|
| clock (GHz) | 1.91 | 1.59 | 1.76 | 2.69 | 4.15 | 4.63 |

30/30 trials slow; trial medians over 2–10 ms between 0.54 and 3.75 GHz
(with an `int8` next load: 0.54–2.94). APERF/MPERF agree. Unlike the
shared buffer, the clock stays high while all 8 threads stream (4.45–4.59)
and collapses **in the tail**: threads that finished early had 4.5 GHz in
their last 2 ms, the last ones 2.84. A likely reason: as threads drop
out, the remaining threads' slices fit in L3, so the tail turns into
L3-resident streaming, the strongest trigger above.

## What was ruled out

| Hypothesis | Test | Verdict |
|---|---|---|
| A current/power limit on a load step (H2 of 0001) | Load step into int8 from idle or from an even end | No: the step starts fast (4.62 GHz) and settles to the ~4.05 GHz limit over ~10 ms, never slower than steady |
| The power limiter holds the clock down afterwards | `int8 uneven -> light` | No: the clock is back at 4.63 GHz at once |
| Memory stalls lower the clock | `chase` | No: 4.61 GHz while chasing, no slowdown afterwards |
| The idle tail is required | `stream even` vs `stream uneven` | No: even ends show it too; uneven deepens it with all threads |
| A clock drop at the transition only | Sensor during the previous phase | No (shared buffer): the clock is already low before the barrier |
| An issue limit, not a clock drop | APERF/MPERF and B/V rates | No: APERF/MPERF match; B/V at theory |

## Conclusion

**Supported by the data:** on this Zen 3, sustained fast streaming reads
into the cores make the **whole chip** lower its clock while they run:
mildly from DRAM (to ~3.7–4.3 GHz), strongly from L3 (to ~2.3 GHz). The
**next phase starts at that clock** and needs 10–15 ms to recover, and up
to 40+ ms after the worst case measured (L3-sized private slices, down to
0.5 GHz). Memory stalls without bandwidth do not trigger it, and a
power-limited compute phase releases immediately. Every thread is slowed,
including those that never slept.

**Hypothesis:** the slow, chip-wide release looks like a firmware control
loop reacting to sustained cache or fabric traffic, separate from the
power and current limiter. Not measured: the mechanism itself, or why
traffic from L3 weighs more than traffic from DRAM.

**Link to finding 0001:** a 512-token attention phase whose K/V fits in
L3 is the strongest trigger measured, and the worst trials (2.5–3.0 GHz
against 4.6) match 0001's 1.5–1.8× over ~10 ms. **Confirmed inside the
engine (2026-10-04):** a clock sensor on every pool thread read, right
after attention, 0.86× (Qwen3-0.6B), **0.14×** (Qwen3-1.7B, median 0.59
GHz, floor 0.54 — the same floor as the private-slices case above) and
0.67× (Qwen3-4B) of the clock before Q/K/V, back to normal by the end of
the layer. Details and raw data in
[finding 0001](../0001-post-attention-slowdown/#confirmation-inside-the-engine-2026-10-04).

**Practical reading for parallel code:** after a memory-bound phase, the
next phase may run well below the clock it would get in steady state, on
every core, for tens of milliseconds. Benchmarks that time a compute phase
in isolation will not see it.

## Open questions

- Bandwidth or location? The same L3-sized buffer read at a throttled rate
  would separate the two.
- The clock during the previous phase is instruction-timed only; an
  APERF/MPERF cross-check of that part is pending.
- Why did idle SMT siblings **spinning** instead of sleeping cut the
  recovery to ~2 ms (`--set stream`: 4/30 slow trials instead of 13/30)?
- Run-to-run variation: the shared DRAM arm was clearly milder in the
  `trigger` session (6/30 slow trials) than in the `stream` session (17/30).
- ~~Does it happen inside the LLM engine, around attention?~~ Yes, see
  finding 0001 (2026-10-04).
- Can the engine avoid it without changing results? Next: fewer K/V
  passes in attention (several query rows per pass) and threads spinning
  instead of sleeping, each measured with the same sensor.
- Other Zen 3 chips, other generations, Intel?

## Replications

| Who | CPU | Result | Link |
|---|---|---|---|
| — | — | — | — |
