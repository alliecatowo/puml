#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/docs/benchmarks"
BIN="$ROOT_DIR/target/release/puml"
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
MODE="full"
RUNS=10
WARMUP=2
FALLBACK_RUNS=12
ENFORCE_GATES=0
UPDATE_BASELINE=0

BINARY_LIMIT_BYTES_FULL=2000000
BINARY_LIMIT_BYTES_QUICK=2500000
ABS_MEAN_LIMIT_MS_FULL=250
ABS_MEAN_LIMIT_MS_QUICK=350
REGRESSION_LIMIT_PCT_FULL=10
REGRESSION_LIMIT_PCT_QUICK=20
REGRESSION_MIN_DELTA_MS_FULL=20
REGRESSION_MIN_DELTA_MS_QUICK=30

usage() {
  cat <<'USAGE'
Usage: ./scripts/bench.sh [--quick] [--dry] [--enforce-gates] [--update-baseline]

Options:
  --quick            fewer runs for fast local validation
  --dry              print resolved scenarios and exit without executing
  --enforce-gates    fail when binary/perf thresholds are exceeded
  --update-baseline  replace mode baseline after successful run
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      MODE="quick"
      RUNS=5
      WARMUP=2
      FALLBACK_RUNS=7
      shift
      ;;
    --dry)
      MODE="dry"
      shift
      ;;
    --enforce-gates)
      ENFORCE_GATES=1
      shift
      ;;
    --update-baseline)
      UPDATE_BASELINE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[bench] unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

mkdir -p "$OUT_DIR"

if [[ "$MODE" == "quick" ]]; then
  BINARY_LIMIT_BYTES="$BINARY_LIMIT_BYTES_QUICK"
  ABS_MEAN_LIMIT_MS="$ABS_MEAN_LIMIT_MS_QUICK"
  REGRESSION_LIMIT_PCT="$REGRESSION_LIMIT_PCT_QUICK"
  REGRESSION_MIN_DELTA_MS="$REGRESSION_MIN_DELTA_MS_QUICK"
  BASELINE_JSON="$OUT_DIR/baseline_quick.json"
else
  BINARY_LIMIT_BYTES="$BINARY_LIMIT_BYTES_FULL"
  ABS_MEAN_LIMIT_MS="$ABS_MEAN_LIMIT_MS_FULL"
  REGRESSION_LIMIT_PCT="$REGRESSION_LIMIT_PCT_FULL"
  REGRESSION_MIN_DELTA_MS="$REGRESSION_MIN_DELTA_MS_FULL"
  BASELINE_JSON="$OUT_DIR/baseline_full.json"
fi

SCENARIOS=(
  "cold_start_help::$BIN --help >/dev/null"
  "parser_check::$BIN --check $ROOT_DIR/tests/fixtures/basic/hello.puml >/dev/null"
  "parser_dump_scene::$BIN --dump scene $ROOT_DIR/tests/fixtures/basic/hello.puml >/dev/null"
  "render_file::$BIN $ROOT_DIR/tests/fixtures/basic/hello.puml --output /tmp/puml-bench-render-$$.svg >/dev/null && rm -f /tmp/puml-bench-render-$$.svg"
  "render_stdin::cat $ROOT_DIR/tests/fixtures/basic/hello.puml | $BIN - >/dev/null"
  "render_stdin_multi::cat $ROOT_DIR/tests/fixtures/structure/multi_three.puml | $BIN --multi - >/dev/null"
)

if [[ "$MODE" == "dry" ]]; then
  echo "[bench] dry run (no execution)"
  echo "[bench] mode: $MODE"
  echo "[bench] binary: $BIN"
  echo "[bench] baseline: $BASELINE_JSON"
  echo "[bench] enforce_gates: $ENFORCE_GATES"
  echo "[bench] update_baseline: $UPDATE_BASELINE"
  echo "[bench] scenarios:"
  for entry in "${SCENARIOS[@]}"; do
    echo "  - ${entry%%::*}: ${entry#*::}"
  done
  exit 0
fi

echo "[bench] building release binary"
cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml" >/dev/null

if command -v hyperfine >/dev/null 2>&1; then
  HAVE_HYPERFINE=1
  TIMING_TOOL="hyperfine"
else
  HAVE_HYPERFINE=0
  TIMING_TOOL="python-perf-counter"
fi

HOST_NAME="$(hostname -s 2>/dev/null || hostname || echo unknown)"
OS_NAME="$(uname -s)"
KERNEL="$(uname -r)"
ARCH="$(uname -m)"
RUSTC_VERSION="$(rustc -V 2>/dev/null || echo unknown)"

CSV="$OUT_DIR/latest.csv"
JSON="$OUT_DIR/latest.json"
MD="$OUT_DIR/latest.md"
TREND_JSON="$OUT_DIR/latest_trend.json"
TREND_MD="$OUT_DIR/latest_trend.md"
PREV_JSON="$OUT_DIR/.baseline.previous.json"

if [[ -f "$BASELINE_JSON" ]]; then
  cp "$BASELINE_JSON" "$PREV_JSON"
else
  rm -f "$PREV_JSON"
fi

echo "name,tool,mean_ms,stddev_ms,runs,timestamp_utc" > "$CSV"
echo "{" > "$JSON"
echo "  \"timestamp_utc\": \"$TS\"," >> "$JSON"
echo "  \"binary\": \"$BIN\"," >> "$JSON"
echo "  \"mode\": \"$MODE\"," >> "$JSON"
echo "  \"environment\": {" >> "$JSON"
echo "    \"host\": \"$HOST_NAME\"," >> "$JSON"
echo "    \"os\": \"$OS_NAME\"," >> "$JSON"
echo "    \"kernel\": \"$KERNEL\"," >> "$JSON"
echo "    \"arch\": \"$ARCH\"," >> "$JSON"
echo "    \"rustc\": \"$RUSTC_VERSION\"," >> "$JSON"
echo "    \"timing_tool\": \"$TIMING_TOOL\"" >> "$JSON"
echo "  }," >> "$JSON"
echo "  \"scenarios\": [" >> "$JSON"

printf '%s\n\n' '# Benchmark Results' > "$MD"
printf '%s\n' "- Timestamp (UTC): \`$TS\`" >> "$MD"
printf '%s\n' "- Binary: \`$BIN\`" >> "$MD"
printf '%s\n' "- Mode: \`$MODE\`" >> "$MD"
printf '%s\n' "- Baseline: \`$BASELINE_JSON\`" >> "$MD"
printf '%s\n' "- Timing tool: \`$TIMING_TOOL\`" >> "$MD"
printf '%s\n' "- Environment: \`$HOST_NAME\` / \`$OS_NAME\` \`$KERNEL\` / \`$ARCH\` / \`$RUSTC_VERSION\`" >> "$MD"
printf '%s\n' "- Gate profile: abs mean <= \`${ABS_MEAN_LIMIT_MS}ms\`, regression <= \`${REGRESSION_LIMIT_PCT}%%\`, binary <= \`${BINARY_LIMIT_BYTES}\` bytes" >> "$MD"
printf '%s\n\n' '- PlantUML comparison: TODO (no-Java environment baseline run)' >> "$MD"
printf '%s\n' '| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |' >> "$MD"
printf '%s\n' '|---|---:|---:|---:|---|' >> "$MD"

measure_with_python_fallback() {
  local cmd="$1"
  local warmup="$2"
  local runs="$3"

  python3 - "$cmd" "$warmup" "$runs" <<'PY'
import statistics
import subprocess
import sys
import time

cmd = sys.argv[1]
warmup = int(sys.argv[2])
runs = int(sys.argv[3])

for _ in range(warmup):
    subprocess.run(["bash", "-lc", cmd], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

samples = []
for _ in range(runs):
    t0 = time.perf_counter_ns()
    subprocess.run(["bash", "-lc", cmd], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    t1 = time.perf_counter_ns()
    samples.append((t1 - t0) / 1_000_000.0)

mean_ms = statistics.mean(samples) if samples else 0.0
std_ms = statistics.pstdev(samples) if len(samples) > 1 else 0.0
print(f"{mean_ms:.3f},{std_ms:.3f},{runs}")
PY
}

first=1
for entry in "${SCENARIOS[@]}"; do
  name="${entry%%::*}"
  cmd="${entry#*::}"

  if [[ "$HAVE_HYPERFINE" -eq 1 ]]; then
    TMP_JSON="$(mktemp)"
    hyperfine --warmup "$WARMUP" --runs "$RUNS" --export-json "$TMP_JSON" "$cmd" >/dev/null
    mean_ms="$(awk -F': ' '/"mean"/ {gsub(/,/, "", $2); printf "%.3f", $2*1000; exit}' "$TMP_JSON")"
    std_ms="$(awk -F': ' '/"stddev"/ {gsub(/,/, "", $2); printf "%.3f", $2*1000; exit}' "$TMP_JSON")"
    runs="$RUNS"
    tool="hyperfine"
    rm -f "$TMP_JSON"
  else
    stats="$(measure_with_python_fallback "$cmd" "$WARMUP" "$FALLBACK_RUNS")"
    IFS=',' read -r mean_ms std_ms runs <<< "$stats"
    tool="python-perf-counter"
  fi

  echo "$name,$tool,$mean_ms,$std_ms,$runs,$TS" >> "$CSV"
  printf '| `%s` | %s | %s | %s | `%s` |\n' "$name" "$mean_ms" "$std_ms" "$runs" "$tool" >> "$MD"

  if [[ "$first" -eq 0 ]]; then
    echo "    ," >> "$JSON"
  fi
  first=0

  cat >> "$JSON" <<REC
    {
      "name": "$name",
      "tool": "$tool",
      "mean_ms": $mean_ms,
      "stddev_ms": $std_ms,
      "runs": $runs
    }
REC
done

echo "  ]" >> "$JSON"
echo "}" >> "$JSON"

BINARY_BYTES="$(stat -c%s "$BIN")"

python3 "$ROOT_DIR/scripts/bench_gate.py" trend \
  --current "$JSON" \
  --previous "$PREV_JSON" \
  --output-json "$TREND_JSON" \
  --output-md "$TREND_MD" \
  --timestamp-utc "$TS" \
  --mode "$MODE" \
  --abs-limit "$ABS_MEAN_LIMIT_MS" \
  --regression-limit-pct "$REGRESSION_LIMIT_PCT" \
  --regression-min-delta-ms "$REGRESSION_MIN_DELTA_MS" \
  --binary-bytes "$BINARY_BYTES" \
  --binary-limit-bytes "$BINARY_LIMIT_BYTES" \
  --host "$HOST_NAME" \
  --os-name "$OS_NAME" \
  --kernel "$KERNEL" \
  --arch "$ARCH" \
  --rustc "$RUSTC_VERSION" \
  --timing-tool "$TIMING_TOOL"

printf '\n%s\n' '## PlantUML Comparison (TODO)' >> "$MD"
printf '%s\n' 'Method when Java is available:' >> "$MD"
printf '%s\n' '1. Run the same fixture set through `puml` and PlantUML.' >> "$MD"
printf '%s\n' '2. Record parse success, render success, and elapsed time per fixture.' >> "$MD"
printf '%s\n' '3. Add comparison rows labeled `plantuml_*` with timestamp + command details.' >> "$MD"

GATE_FAILURES=()

echo "[bench] gate profile: mode=$MODE abs_mean<=${ABS_MEAN_LIMIT_MS}ms regression<=${REGRESSION_LIMIT_PCT}%+>${REGRESSION_MIN_DELTA_MS}ms binary<=${BINARY_LIMIT_BYTES}B"
while IFS= read -r failure; do
  if [[ -n "$failure" ]]; then
    GATE_FAILURES+=("$failure")
  fi
done < <(python3 "$ROOT_DIR/scripts/bench_gate.py" failures \
  --current "$JSON" \
  --previous "$PREV_JSON" \
  --mode "$MODE" \
  --abs-limit "$ABS_MEAN_LIMIT_MS" \
  --regression-limit-pct "$REGRESSION_LIMIT_PCT" \
  --regression-min-delta-ms "$REGRESSION_MIN_DELTA_MS" \
  --binary-bytes "$BINARY_BYTES" \
  --binary-limit-bytes "$BINARY_LIMIT_BYTES")

if [[ "$ENFORCE_GATES" -eq 1 && "${#GATE_FAILURES[@]}" -gt 0 ]]; then
  echo "[bench] gate failures:" >&2
  for failure in "${GATE_FAILURES[@]}"; do
    echo "  - $failure" >&2
  done
  echo "[bench] trend: $TREND_JSON" >&2
  exit 1
fi

if [[ "${#GATE_FAILURES[@]}" -gt 0 ]]; then
  echo "[bench] gate warnings (not enforced):"
  for failure in "${GATE_FAILURES[@]}"; do
    echo "  - $failure"
  done
else
  echo "[bench] gates: pass"
fi

if [[ "$UPDATE_BASELINE" -eq 1 ]]; then
  cp "$JSON" "$BASELINE_JSON"
  echo "[bench] baseline updated: $BASELINE_JSON"
fi

echo "[bench] wrote:"
echo "  - $CSV"
echo "  - $JSON"
echo "  - $MD"
echo "  - $TREND_JSON"
echo "  - $TREND_MD"

rm -f "$PREV_JSON"
