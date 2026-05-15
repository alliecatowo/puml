#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/docs/benchmarks"
BIN="$ROOT_DIR/target/release/puml"
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
MODE="default"
RUNS=10
WARMUP=2
FALLBACK_RUNS=5

usage() {
  cat <<'USAGE'
Usage: ./scripts/bench.sh [--quick] [--dry]

Options:
  --quick  fewer runs for fast local validation
  --dry    print resolved scenarios and exit without executing
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
  echo "[bench] scenarios:"
  for entry in "${SCENARIOS[@]}"; do
    echo "  - ${entry%%::*}: ${entry#*::}"
  done
  exit 0
fi

CSV="$OUT_DIR/latest.csv"
JSON="$OUT_DIR/latest.json"
MD="$OUT_DIR/latest.md"

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

printf '\n%s\n' '## PlantUML Comparison (TODO)' >> "$MD"
printf '%s\n' 'Method when Java is available:' >> "$MD"
printf '%s\n' '1. Run the same fixture set through `puml` and PlantUML.' >> "$MD"
printf '%s\n' '2. Record parse success, render success, and elapsed time per fixture.' >> "$MD"
printf '%s\n' '3. Add comparison rows labeled `plantuml_*` with timestamp + command details.' >> "$MD"

echo "[bench] wrote:"
echo "  - $CSV"
echo "  - $JSON"
echo "  - $MD"
