# lab-notes

*Leia em português: [README.pt-BR.md](README.pt-BR.md)*

Measured findings from my own projects: how CPUs really behave under
real workloads, and how my CPU inference engine compares with existing
tools. Every finding comes with its protocol, the machine it ran on and
the raw numbers, so you can check it or run it on your own hardware.

By [Fabio Henrique](https://github.com/fabious054) (CodeWave.IT).

## Areas

| Folder | What goes there |
|---|---|
| [`cpu/`](cpu/) | Hardware behaviour: clocks, caches, cores, power management, measurement pitfalls |
| [`inference/`](inference/) | Results of CandleCLI (private for now), a CPU LLM inference engine written from scratch in pure Rust, measured against other tools |

## Findings

| # | Finding | Area | Status |
|---|---|---|---|
| 0001 | [Cores run 1.5–1.8× slower for ~10 ms after an unevenly ending parallel phase](cpu/0001-post-attention-slowdown/) | cpu | Single machine, mechanism (0004) confirmed and mitigated in the engine |
| 0002 | [APERF/MPERF are readable from user mode on Windows through RDPRU](cpu/0002-rdpru-user-mode-windows/) | cpu | Single machine |
| 0003 | [A core that sleeps more than ~1–2 ms restarts ~20% slower for up to 1 ms](cpu/0003-core-wake-ramp/) | cpu | Single machine |
| 0004 | [Fast streaming reads lower the whole chip's clock, and the next phase pays for 10–40 ms](cpu/0004-post-streaming-clock-depression/) | cpu | Two machines (Zen 3; L3 trigger also on Intel Tiger Lake-H) |
| 0005 | [CandleCLI vs llama.cpp b11382 on Qwen3 Q8_0: faster prompt reading on 1.7B and 4B, 2–6% behind on generation](inference/0005-qwen3-vs-llama-cpp-b11382/) | inference | Single machine |

## How to read a finding

Each finding is a folder with a `README.md` in a fixed format
([TEMPLATE.md](TEMPLATE.md)):

- **Question**: what was being asked.
- **Machine**: CPU, memory, OS, toolchain.
- **Protocol**: how it was measured (see [PROTOCOL.md](PROTOCOL.md) for the
  rules every measurement follows).
- **Results**: the numbers, with the raw output in `data/`.
- **Conclusion**: what the data supports, and what it does not.
- **Status**: `draft`, `single machine`, `replicated by N`, or `refuted`.

## What is (and is not) published here

Findings about hardware, and the measurement method, are published in
full. For the inference engine, results are published (machine, model,
the exact command used for the other tool, raw numbers); implementation
details of the engine are not.

## Run it on your machine

The CPU findings are measured with Ember, my CPU lab, which is private
for now; a public release is planned, and with it a ready-to-run probe
for each finding. Until then, some findings include the probe code
itself (for example [0001](cpu/0001-post-attention-slowdown/probe.rs)
and [0002](cpu/0002-rdpru-user-mode-windows/rdpru.rs)), and every
finding describes its protocol in enough detail to write your own.

If you reproduce a finding, or fail to, open an issue with the
"Replication" template. A result that disagrees is as useful as one that
agrees.

## License

[MIT](LICENSE) — code and data.
