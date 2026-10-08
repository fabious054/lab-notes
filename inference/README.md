# inference

Results of CandleCLI (private for now), my own CPU LLM inference
engine in Rust (no Python, llama.cpp or Ollama at runtime), measured
against other tools on the same machine.

Each result records the machine, the model file, the exact command used
for the other tool, and the raw numbers, following
[PROTOCOL.md](../PROTOCOL.md). Engine implementation details are not
published here.

| # | Result | Status |
|---|---|---|
| 0005 | [CandleCLI vs llama.cpp b11382 on Qwen3 Q8_0: faster prompt reading on 1.7B and 4B, 2–6% behind on generation](0005-qwen3-vs-llama-cpp-b11382/) | Single machine |
