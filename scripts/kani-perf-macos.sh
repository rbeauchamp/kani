#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Apple Silicon solver benchmark (issue #6). Implements the pre-registered
# protocol posted to https://github.com/rbeauchamp/kani/issues/6 (with the
# fixture-sourcing amendment recorded there):
#   - Solvers: minisat (AC3 baseline), cadical (Kani default), kissat (AC3 candidate)
#   - N rounds, round-robin interleaved across solvers to cancel thermal drift
#   - Primary metric: CBMC-reported "Runtime decision procedure" (solver wall-clock)
#   - Secondary metric: Kani "Verification Time" (end-to-end harness wall-clock)
#   - Success (AC3): median(kissat)/median(minisat) <= 0.80 on the aggregate
#     AND kissat faster in >= 80% of paired runs per fixture
#
# Fixture-sourcing note (measured, 2026-09-08): naive synthetic SAT fixtures
# (bijective-mixer injectivity, multiplier inversion, 64-bit factorization,
# deep-unwind recurrences, symbolic alias partitions) are collapsed by CBMC's
# simplifier to <=0.1s solver time; the genuinely solver-bound small harnesses
# in the public tree are container-probing proofs. The benchmark therefore uses
# the two solver-dominated public harnesses that land in the 5-30s band.
#
# Usage: ./scripts/kani-perf-macos.sh [rounds]   (default 10 rounds)
# Requires: dev build present (cargo build-dev), python3 on PATH.

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
KANI_DIR=$SCRIPT_DIR/..
PERF_DIR="${KANI_DIR}/tests/perf"
ROUNDS="${1:-10}"
PER_RUN_TIMEOUT="180s"
OUT_DIR="${PERF_DIR}/macos_arm64"
RAW_CSV="${OUT_DIR}/raw.csv"
REPORT_MD="${OUT_DIR}/REPORT.md"

export PATH="${KANI_DIR}/scripts:${HOME}/.local/bin:/opt/homebrew/bin:${PATH}"

# dir:relative-to-tests/perf : harness : extra kani flags
CASES=(
    "btreeset/insert_same:main:"
    "hashset:check_insert:-Z stubbing"
)
SOLVERS=(minisat cadical kissat)

mkdir -p "${OUT_DIR}"
{
    echo "# Copyright Kani Contributors"
    echo "# SPDX-License-Identifier: Apache-2.0 OR MIT"
    echo "round,fixture,harness,solver,solver_s,verify_s,status"
} > "${RAW_CSV}"

for round in $(seq 1 "${ROUNDS}"); do
    echo "== round ${round}/${ROUNDS} =="
    for case in "${CASES[@]}"; do
        dir="$(cut -d: -f1 <<<"${case}")"
        harness="$(cut -d: -f2 <<<"${case}")"
        extra="$(cut -d: -f3- <<<"${case}")"
        for solver in "${SOLVERS[@]}"; do
            out="$(cd "${PERF_DIR}/${dir}" && timeout 240 \
                    cargo kani -Z unstable-options --solver "${solver}" --harness "${harness}" \
                    --harness-timeout "${PER_RUN_TIMEOUT}" ${extra} 2>&1)" || true
            solver_s="$(grep -oE 'Runtime decision procedure: [0-9.e+-]+' <<<"${out}" \
                        | grep -oE '[0-9.e+-]+$' \
                        | awk '{s+=$1} END {if (s>0) printf "%.6f", s}' || true)"
            verify_s="$(grep -oE 'Verification Time: [0-9.e+-]+' <<<"${out}" \
                        | grep -oE '[0-9.e+-]+$' | tail -1 || true)"
            status="$(grep -oE 'VERIFICATION:- [A-Z]+' <<<"${out}" | tail -1 \
                       | awk '{print $2}' || true)"
            echo "${round},${dir},${harness},${solver},${solver_s:-NA},${verify_s:-NA},${status:-TIMEOUT}" \
                >> "${RAW_CSV}"
        done
    done
done

python3 - "${RAW_CSV}" "${REPORT_MD}" "${ROUNDS}" <<'PYEOF'
import csv, statistics, sys, collections, datetime

raw_path, report_path, rounds = sys.argv[1], sys.argv[2], int(sys.argv[3])
rows = list(csv.DictReader(line for line in open(raw_path) if not line.startswith("#")))

def val(r):
    try: return float(r["solver_s"])
    except ValueError: return None

fixtures = sorted({r["harness"] for r in rows})
solvers = ["minisat", "cadical", "kissat"]

per = collections.defaultdict(list)
paired = collections.defaultdict(dict)  # (harness, round) -> {solver: s}
for r in rows:
    v = val(r)
    if v is None: continue
    per[(r["harness"], r["solver"])].append(v)
    paired[(r["harness"], r["round"])][r["solver"]] = v

def med(h, s):
    xs = per.get((h, s), [])
    return statistics.median(xs) if xs else None

def iqr(h, s):
    xs = sorted(per.get((h, s), []))
    if len(xs) < 4:
        return None
    q = statistics.quantiles(xs, n=4, method="inclusive")
    return q[0], q[2]

lines = []
lines.append("# Apple Silicon solver benchmark report (issue #6)\n")
lines.append(f"Generated: {datetime.datetime.now().isoformat(timespec='seconds')} "
             f"({rounds} rounds, round-robin interleaved)\n")
lines.append("Primary metric: CBMC `Runtime decision procedure` seconds, summed across "
             "solver invocations in the run (median of N rounds; IQR in brackets).\n")
lines.append("| fixture | minisat | cadical | kissat | kissat/minisat | kissat/cadical | kissat-wins-vs-minisat |")
lines.append("|---|---|---|---|---|---|---|")
agg = collections.defaultdict(list)
ac3_ok = True
for h in fixtures:
    m, c, k = med(h, "minisat"), med(h, "cadical"), med(h, "kissat")
    if m is None or k is None:
        lines.append(f"| {h} | {m} | {c} | {k} | n/a | n/a | n/a |")
        ac3_ok = False
        continue
    def cell(hh, ss, v):
        r = iqr(hh, ss)
        return f"{v:.3f} [{r[0]:.3f}–{r[1]:.3f}]" if r else f"{v:.3f}"
    wins = sum(1 for (hh, _), d in paired.items()
               if hh == h and "kissat" in d and "minisat" in d and d["kissat"] < d["minisat"])
    total = sum(1 for (hh, _), d in paired.items()
                if hh == h and "kissat" in d and "minisat" in d)
    ratio_m = k / m
    ratio_c = (k / c) if c else float("nan")
    agg["minisat"] += per[(h, "minisat")]
    agg["cadical"] += per[(h, "cadical")]
    agg["kissat"] += per[(h, "kissat")]
    lines.append(f"| {h} | {cell(h, 'minisat', m)} | {cell(h, 'cadical', c)} | {cell(h, 'kissat', k)} | {ratio_m:.3f} | {ratio_c:.3f} | {wins}/{total} |")
    # Pre-registered criterion: per-fixture paired-win rate >= 80% (aggregate ratio is
    # judged below, on the pooled medians).
    if total == 0 or wins < 0.8 * total:
        ac3_ok = False

am = statistics.median(agg["minisat"]) if agg["minisat"] else None
ak = statistics.median(agg["kissat"]) if agg["kissat"] else None
ac = statistics.median(agg["cadical"]) if agg["cadical"] else None
lines.append("")
lines.append("## Aggregate (all fixtures pooled)\n")
if am and ak:
    lines.append(f"- median minisat: {am:.3f}s; median cadical: {ac:.3f}s; median kissat: {ak:.3f}s")
    lines.append(f"- aggregate kissat/minisat ratio: {ak/am:.3f} (AC3 threshold: <= 0.800)")
    lines.append(f"- aggregate kissat/cadical ratio: {ak/ac:.3f} (secondary; cadical is Kani's default)")
    lines.append("")
    lines.append(f"**AC3 (>=20% solving speedup vs built-in MiniSAT): "
                 f"{'MET' if ac3_ok and ak/am <= 0.80 else 'NOT MET'}** (judged exactly as written)")
open(report_path, "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
PYEOF

echo "Report written to ${REPORT_MD}"
