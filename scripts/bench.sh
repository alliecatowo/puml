#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/docs/benchmarks"
BIN="$ROOT_DIR/target/release/puml"
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
MODE="full"
RUNS=10
WARMUP=2
FALLBACK_RUNS=5
ENFORCE_GATES=0

BINARY_LIMIT_BYTES_FULL=2000000
BINARY_LIMIT_BYTES_QUICK=2500000
ABS_MEAN_LIMIT_MS_FULL=250
ABS_MEAN_LIMIT_MS_QUICK=350
REGRESSION_LIMIT_PCT_FULL=10
REGRESSION_LIMIT_PCT_QUICK=20

usage() {
  cat <<'USAGE'
Usage: ./scripts/bench.sh [--quick] [--dry] [--enforce-gates]

Options:
  --quick          fewer runs for fast local validation
  --dry            print resolved scenarios and exit without executing
  --enforce-gates  fail when binary/perf thresholds are exceeded
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      MODE="quick"
      RUNS=3
      WARMUP=1
      FALLBACK_RUNS=3
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
else
  BINARY_LIMIT_BYTES="$BINARY_LIMIT_BYTES_FULL"
  ABS_MEAN_LIMIT_MS="$ABS_MEAN_LIMIT_MS_FULL"
  REGRESSION_LIMIT_PCT="$REGRESSION_LIMIT_PCT_FULL"
fi

echo "[bench] building release binary"
cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml" >/dev/null

if command -v hyperfine >/dev/null 2>&1; then
  HAVE_HYPERFINE=1
else
  HAVE_HYPERFINE=0
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
  echo "[bench] enforce_gates: $ENFORCE_GATES"
  echo "[bench] scenarios:"
  for entry in "${SCENARIOS[@]}"; do
    echo "  - ${entry%%::*}: ${entry#*::}"
  done
  exit 0
fi

CSV="$OUT_DIR/latest.csv"
JSON="$OUT_DIR/latest.json"
MD="$OUT_DIR/latest.md"
TREND_JSON="$OUT_DIR/latest_trend.json"
TREND_MD="$OUT_DIR/latest_trend.md"
PREV_JSON="$OUT_DIR/.latest.previous.json"

if [[ -f "$JSON" ]]; then
  cp "$JSON" "$PREV_JSON"
else
  rm -f "$PREV_JSON"
fi

echo "name,tool,mean_ms,stddev_ms,runs,timestamp_utc" > "$CSV"
echo "{" > "$JSON"
echo "  \"timestamp_utc\": \"$TS\"," >> "$JSON"
echo "  \"binary\": \"$BIN\"," >> "$JSON"
echo "  \"mode\": \"$MODE\"," >> "$JSON"
echo "  \"scenarios\": [" >> "$JSON"

printf '%s\n\n' '# Benchmark Results' > "$MD"
printf '%s\n' "- Timestamp (UTC): \`$TS\`" >> "$MD"
printf '%s\n' "- Binary: \`$BIN\`" >> "$MD"
printf '%s\n' "- Mode: \`$MODE\`" >> "$MD"
printf '%s\n' "- Gate profile: abs mean <= \`${ABS_MEAN_LIMIT_MS}ms\`, regression <= \`${REGRESSION_LIMIT_PCT}%%\`, binary <= \`${BINARY_LIMIT_BYTES}\` bytes" >> "$MD"
printf '%s\n\n' '- PlantUML comparison: TODO (no-Java environment baseline run)' >> "$MD"
printf '%s\n' '| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |' >> "$MD"
printf '%s\n' '|---|---:|---:|---:|---|' >> "$MD"

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
    times=()
    for _ in $(seq 1 "$FALLBACK_RUNS"); do
      t="$(/usr/bin/time -f '%e' bash -lc "$cmd" 2>&1 >/dev/null | tail -n1)"
      times+=("$t")
    done
    mean_ms="$(printf '%s\n' "${times[@]}" | awk '{sum+=$1; n+=1} END {if (n==0) print "0.000"; else printf "%.3f", (sum/n)*1000}')"
    std_ms="$(printf '%s\n' "${times[@]}" | awk '{x[NR]=$1; sum+=$1} END {if (NR==0) {print "0.000"; exit} m=sum/NR; for(i=1;i<=NR;i++){d=x[i]-m; ss+=d*d} printf "%.3f", sqrt(ss/NR)*1000}')"
    runs="$FALLBACK_RUNS"
    tool="time"
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

python3 - "$JSON" "$PREV_JSON" "$TREND_JSON" "$TREND_MD" "$TS" "$MODE" "$ABS_MEAN_LIMIT_MS" "$REGRESSION_LIMIT_PCT" "$BINARY_BYTES" "$BINARY_LIMIT_BYTES" <<'PY'
import json
import pathlib
import sys

json_path = pathlib.Path(sys.argv[1])
prev_path = pathlib.Path(sys.argv[2])
trend_json_path = pathlib.Path(sys.argv[3])
trend_md_path = pathlib.Path(sys.argv[4])
ts = sys.argv[5]
mode = sys.argv[6]
abs_limit = float(sys.argv[7])
regression_limit_pct = float(sys.argv[8])
binary_bytes = int(sys.argv[9])
binary_limit_bytes = int(sys.argv[10])

current = json.loads(json_path.read_text())
prev = None
if prev_path.exists() and prev_path.stat().st_size > 0:
    prev = json.loads(prev_path.read_text())

prev_means = {}
if prev:
    for item in prev.get("scenarios", []):
        prev_means[item["name"]] = float(item["mean_ms"])

rows = []
for item in current.get("scenarios", []):
    name = item["name"]
    curr = float(item["mean_ms"])
    prev_val = prev_means.get(name)
    delta_ms = None if prev_val is None else round(curr - prev_val, 3)
    delta_pct = None
    if prev_val not in (None, 0.0):
        delta_pct = round(((curr - prev_val) / prev_val) * 100.0, 3)
    rows.append(
        {
            "name": name,
            "current_mean_ms": round(curr, 3),
            "previous_mean_ms": None if prev_val is None else round(prev_val, 3),
            "delta_ms": delta_ms,
            "delta_pct": delta_pct,
        }
    )

rows.sort(key=lambda r: r["name"])

trend = {
    "timestamp_utc": ts,
    "mode": mode,
    "source": "docs/benchmarks/latest.json",
    "binary": {
        "path": current.get("binary"),
        "size_bytes": binary_bytes,
        "limit_bytes": binary_limit_bytes,
        "within_limit": binary_bytes <= binary_limit_bytes,
    },
    "gates": {
        "absolute_mean_ms_limit": abs_limit,
        "regression_pct_limit": regression_limit_pct,
    },
    "scenarios": rows,
    "baseline": {
        "timestamp_utc": None if prev is None else prev.get("timestamp_utc"),
        "available": prev is not None,
    },
    "plantuml_oracle": {
        "status": "todo",
        "notes": "No-Java baseline keeps oracle placeholders only.",
    },
}

trend_json_path.write_text(json.dumps(trend, indent=2, sort_keys=True) + "\n")

lines = [
    "# Benchmark Trend",
    "",
    f"- Timestamp (UTC): `{ts}`",
    f"- Mode: `{mode}`",
    f"- Baseline timestamp (UTC): `{trend['baseline']['timestamp_utc'] or 'none'}`",
    f"- Binary: `{binary_bytes}` bytes (limit `{binary_limit_bytes}`)",
    "",
    "| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |",
    "|---|---:|---:|---:|---:|",
]
for row in rows:
    prev_mean = "n/a" if row["previous_mean_ms"] is None else f"{row['previous_mean_ms']:.3f}"
    delta_ms = "n/a" if row["delta_ms"] is None else f"{row['delta_ms']:.3f}"
    delta_pct = "n/a" if row["delta_pct"] is None else f"{row['delta_pct']:.3f}"
    lines.append(
        f"| `{row['name']}` | {row['current_mean_ms']:.3f} | {prev_mean} | {delta_ms} | {delta_pct} |"
    )

lines.extend(
    [
        "",
        "## PlantUML Oracle",
        "- Status: `todo`",
        "- Notes: no-Java baseline keeps oracle placeholders only.",
    ]
)
trend_md_path.write_text("\n".join(lines) + "\n")
PY

printf '\n%s\n' '## PlantUML Comparison (TODO)' >> "$MD"
printf '%s\n' 'Method when Java is available:' >> "$MD"
printf '%s\n' '1. Run the same fixture set through `puml` and PlantUML.' >> "$MD"
printf '%s\n' '2. Record parse success, render success, and elapsed time per fixture.' >> "$MD"
printf '%s\n' '3. Add comparison rows labeled `plantuml_*` with timestamp + command details.' >> "$MD"

GATE_FAILURES=()

echo "[bench] gate profile: mode=$MODE abs_mean<=${ABS_MEAN_LIMIT_MS}ms regression<=${REGRESSION_LIMIT_PCT}% binary<=${BINARY_LIMIT_BYTES}B"
if (( BINARY_BYTES > BINARY_LIMIT_BYTES )); then
  GATE_FAILURES+=("binary size ${BINARY_BYTES}B exceeds ${BINARY_LIMIT_BYTES}B")
fi

if [[ -f "$PREV_JSON" ]]; then
  while IFS= read -r failure; do
    GATE_FAILURES+=("$failure")
  done < <(python3 - "$JSON" "$PREV_JSON" "$ABS_MEAN_LIMIT_MS" "$REGRESSION_LIMIT_PCT" <<'PY'
import json
import sys

current = json.load(open(sys.argv[1]))
previous = json.load(open(sys.argv[2]))
abs_limit = float(sys.argv[3])
reg_limit = float(sys.argv[4])

prev_map = {item["name"]: float(item["mean_ms"]) for item in previous.get("scenarios", [])}

for item in current.get("scenarios", []):
    name = item["name"]
    curr = float(item["mean_ms"])
    if curr > abs_limit:
        print(f"{name}: mean {curr:.3f}ms exceeds absolute limit {abs_limit:.3f}ms")
    prev = prev_map.get(name)
    if prev is not None and prev > 0:
        delta_pct = ((curr - prev) / prev) * 100.0
        if delta_pct > reg_limit:
            print(f"{name}: regression {delta_pct:.3f}% exceeds limit {reg_limit:.3f}% (current {curr:.3f}ms vs previous {prev:.3f}ms)")
PY
)
else
  while IFS= read -r failure; do
    GATE_FAILURES+=("$failure")
  done < <(python3 - "$JSON" "$ABS_MEAN_LIMIT_MS" <<'PY'
import json
import sys

current = json.load(open(sys.argv[1]))
abs_limit = float(sys.argv[2])

for item in current.get("scenarios", []):
    curr = float(item["mean_ms"])
    if curr > abs_limit:
        print(f"{item['name']}: mean {curr:.3f}ms exceeds absolute limit {abs_limit:.3f}ms")
PY
)
fi

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

echo "[bench] wrote:"
echo "  - $CSV"
echo "  - $JSON"
echo "  - $MD"
echo "  - $TREND_JSON"
echo "  - $TREND_MD"

rm -f "$PREV_JSON"
