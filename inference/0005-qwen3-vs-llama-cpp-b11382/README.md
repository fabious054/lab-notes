# 0005 — CandleCLI vs llama.cpp b11382 on Qwen3 Q8_0: faster prompt reading on 1.7B and 4B, 2–6% behind on generation

**Status:** single machine
**Area:** inference
**Date:** 2026-10-04

## Question

How does CandleCLI, my own CPU inference engine in Rust (private for
now), compare with the latest llama.cpp on the same
machine, the same model files and the same thread counts?

## Machine

| Item | Value |
|---|---|
| CPU | AMD Ryzen 7 5700X (Zen 3), 8 cores / 16 threads, AVX2 + FMA, 32 MB L3 |
| Memory | DDR4-2400, dual channel (2 × 16 GB) |
| OS / power plan | Windows 11; power plan not recorded (Balanced is the default) |
| llama.cpp | b11382 (commit `11fe02151`), official Windows build, Clang 20.1.8, CPU backend `ggml-cpu-haswell` (AVX2) |
| CandleCLI | 0.1.0, `main` of 2026-10-04 (Rust, MSVC, release build), "fast" mode |
| Models | Qwen3-0.6B, 1.7B and 4B, Q8_0 GGUF, the same files for both tools |
| Threads | 16 for prompt reading; 4 and 8 for generation |

## Protocol

- Prompt reading: `llama-bench -p 512 -n 0 -t 16 -r 3` against CandleCLI
  `/bench 512 0 16` (3 reps), twice per side.
- Generation from an empty context: `llama-bench -p 0 -n 128 -t 4 -r 3`
  (and `-t 8`) against `/bench 0 128 4` (and `8`), once per side and
  thread count.
- Every run in a fresh process, after **~2 minutes idle**, alternating
  llama.cpp → CandleCLI ([PROTOCOL.md](../../PROTOCOL.md) rules 2 and 3).
  24 runs from 04:07 to 05:04.
- Both tools quantize activations to 8 bits for the Q8_0 matrix
  products: CandleCLI's "fast" mode uses int8 activations, and
  llama.cpp's Q8_0 path quantizes them to q8_0.

Raw output of every run: [`data/sidebyside-raw.txt`](data/sidebyside-raw.txt).

## Results

**Prompt reading, pp512, 16 threads** (tok/s, mean of the two runs per side):

| Model | llama.cpp b11382 | CandleCLI | CandleCLI vs llama.cpp |
|---|---|---|---|
| Qwen3-0.6B | 524.3 (520.2, 528.3) | 544.6 (542.1, 547.0) | +3.9% |
| Qwen3-1.7B | 181.2 (179.0, 183.5) | 210.0 (208.9, 211.2) | **+15.9%** |
| Qwen3-4B | 72.4 (73.2, 71.6) | 84.8 (84.7, 84.9) | **+17.1%** |

**Generation, tg128 from an empty context** (tok/s, 4 / 8 threads):

| Model | llama.cpp b11382 | CandleCLI | CandleCLI vs llama.cpp |
|---|---|---|---|
| Qwen3-0.6B | 47.40 / 46.47 | 44.98 / 43.87 | 95% / 94% |
| Qwen3-1.7B | 17.43 / 17.04 | 17.11 / 16.39 | 98% / 96% |
| Qwen3-4B | 7.58 / 7.32 | 7.44 / 7.10 | 98% / 97% |

- On 0.6B prompt reading, CandleCLI's run-to-run spread (± 31–33 tok/s
  within each run) is larger than the 3.9% gap: a tie.
- Generation on this machine is limited by memory bandwidth for both
  tools (CandleCLI reads its weights at ~28–32 GB/s, against ~34 GB/s
  measured for the memory).

## What was ruled out

| Hypothesis | Test | Verdict |
|---|---|---|
| Drift over the session favours one side | Runs alternated, ~2 min idle before each; the two pp512 runs per side agree within 2.5% | No |
| Different model files or quantization | Same GGUF files, both Q8_0 with 8-bit activations | Not a factor |

## Conclusion

**Supported by the data, on this machine:** CandleCLI reads a 512-token
prompt **16–17% faster** than llama.cpp b11382 on Qwen3-1.7B and 4B, and
ties on 0.6B. Generation from an empty context is **2–6% slower**.

**Not claimed:** anything about other CPUs, other models, longer
contexts, generation after a long prompt, or memory use. Those were
not measured here.

CandleCLI is private for now, so its side cannot be reproduced yet; the
llama.cpp side can, with the commands above. How the engine works is
not published here (see [What is published](../../README.md#what-is-and-is-not-published-here)).

## Open questions

- Memory use, measured the same way on both sides (working set read
  from the OS counters by a program, see PROTOCOL.md rule 7).
- The same comparison on a laptop and on 4- and 2-thread machines.
- Generation after a long prompt, where attention over the context
  starts to matter.

## Replications

| Who | CPU | Result | Link |
|---|---|---|---|
| — | — | — | — |
