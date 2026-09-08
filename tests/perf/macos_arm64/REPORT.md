# Apple Silicon solver benchmark report (issue #6)

Generated: 2026-09-08T14:38:35 (10 rounds, round-robin interleaved)

Primary metric: CBMC `Runtime decision procedure` seconds, summed across solver invocations in the run (median of N rounds; IQR in brackets).

| fixture | minisat | cadical | kissat | kissat/minisat | kissat/cadical | kissat-wins-vs-minisat |
|---|---|---|---|---|---|---|
| check_insert | 21.535 [21.490–21.742] | 1.798 [1.783–1.808] | 3.903 [3.866–3.941] | 0.181 | 2.171 | 10/10 |
| main | 8.041 [7.968–8.287] | 3.340 [3.261–3.398] | 7.416 [7.364–7.453] | 0.922 | 2.220 | 10/10 |

## Aggregate (all fixtures pooled)

- median minisat: 14.903s; median cadical: 2.488s; median kissat: 5.573s
- aggregate kissat/minisat ratio: 0.374 (AC3 threshold: <= 0.800)
- aggregate kissat/cadical ratio: 2.240 (secondary; cadical is Kani's default)

**AC3 (>=20% solving speedup vs built-in MiniSAT): MET** (judged exactly as written)

## Interpretation (written after the run, per the pre-registered decision criteria)

- **AC3 as written is MET**: Kissat 4.0.1 solves the SAT-heavy fixtures in 0.374× MiniSAT's
  median time (a ~63% reduction, well past the 0.80 threshold), and is faster in 10/10 paired
  runs on both fixtures. IQRs are ±1–3%, so the difference is far outside noise on this machine.
- **No optimization is justified by this result.** Kani's default solver is CaDiCaL, not
  MiniSAT, and Kissat is **2.24× slower than CaDiCaL** on the same fixtures (CaDiCaL also wins
  10/10 paired runs against Kissat — visible in the per-fixture ratios). Changing the default,
  or steering users to `--solver kissat` for this workload class, would regress solving
  performance. The MiniSAT comparison measures a baseline Kani does not use.
- **What this establishes**: (a) on Apple Silicon, the bundled solver trio performs as the
  upstream default configuration intends — CaDiCaL first, Kissat second, MiniSAT last — for
  container/SAT-heavy harnesses; (b) the benchmark protocol is reproducible via
  `./scripts/kani-perf-macos.sh` (raw data: `raw.csv` next to this report).
- **Uncertainty and scope**: single M-series host, two public fixtures, solver time as
  reported by CBMC. Results support no claim about other hardware, other workload classes
  (the fixture-sourcing amendment to issue #6 records why synthetic arithmetic/unwind/alias
  classes could not be made solver-bound), or end-to-end verification time, where CBMC's
  symbolic execution — not the solver — dominates on most harnesses.
