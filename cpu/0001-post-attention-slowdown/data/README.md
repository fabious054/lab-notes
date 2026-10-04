# Raw output — finding 0001

Console output of each diagnostic round, copied verbatim. Only compiler
warnings and local paths were removed.

| File | Round | What it tested |
|---|---|---|
| `v1.txt` | v1 | Weights: `q_proj` vs `o_proj` (and `ffn_down`) in isolation, same input |
| `v2.txt` | v2 | Input data vs position: in-forward times, real inputs re-timed in isolation |
| `v3.txt` | v3 | Input conversion vs multiplication; multiply on a single-thread copy |
| `v4.txt` | v4 | Attention head timeline; two runs: Balanced and High performance power plans |
| `v5.txt` | v5 | Output buffer alignment (+0/16/32/48 bytes) |
| `v6.txt` | v6 | `q_proj` run right after attention; per-task timelines; 0.3 ms spin first |
| `v7.txt` | v7 | Memory-free probes (integer chain, AVX2 FMA) at three points |
| `v8-clock.txt` | v8 | CandleCLI `/bench 512 0 --clock`: core clock of every pool thread at five points per layer, 0.6B/1.7B/4B (2026-10-04) |
| `v9-kv-tiles.txt` | v9 | K/V tiles in attention (1, 4, 8 rows per K/V pass), clock and stage times, 0.6B/1.7B/4B (2026-10-04, extract) |

Line formats:

- `v4`: `h<head>@t<thread>:<start ms>-<end ms>`, relative to the start of the attention phase.
- `v6`: `t<thread>:<tasks>x<mean ms per task>@<first start ms>`.
