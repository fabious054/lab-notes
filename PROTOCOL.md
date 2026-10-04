# Measurement protocol

Every number in this repository follows these rules. A finding that
breaks one says so, and why.

## 1. Know the machine

Record, for every run:

- CPU model, core/thread count, cache sizes;
- memory type and speed;
- OS and version, power plan;
- toolchain and build mode (always an optimized release build);
- thread count used by the workload.

## 2. Control temperature

Back-to-back runs measure how hot the CPU is, not the code. On the
reference machine, the same unchanged program lost about 6% just from
running sessions with no pause.

- Pause **about 2 minutes** before each session.
- Pause **about 1 minute** between benchmarks inside a session, or wait
  until CPU usage has fully dropped.
- Close heavy background programs.

## 3. Alternate A and B

When comparing two versions (or two tools), run them alternately:
A, B, A, B… never all A runs followed by all B runs. Drift over time
(temperature, background load) then hits both sides equally.

## 4. Treat ±2% as noise

On the reference machine, identical code varies by about ±2% between
sessions. A difference inside that band is not a result. Larger claims
need several alternated sessions.

## 5. Keep the raw output

The raw console output goes into the finding's `data/` folder, unedited
except for removing unrelated noise (compiler warnings, local paths).
Tables in the write-up are derived from it, never the other way around.

## 6. Separate what was measured from what is believed

Each conclusion is marked as either supported by the data or a
hypothesis. Hypotheses stay open until a measurement settles them.
