#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/docs/benchmarks"
BIN="$ROOT_DIR/target/release/puml"
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

mkdir -p "$OUT_DIR"

echo "[bench] building release binary"
cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml" >/dev/null

if command -v hyperfine >/dev/null 2>&1; then
  HAVE_HYPERFINE=1
else
  HAVE_HYPERFINE=0
fi

SCENARIOS=(
  "render_hello::$BIN $ROOT_DIR/tests/fixtures/basic/hello.puml"
  "check_hello::$BIN --check $ROOT_DIR/tests/fixtures/basic/hello.puml"
  "dump_model::$BIN --dump model $ROOT_DIR/tests/fixtures/basic/hello.puml"
  "stdin_single::cat $ROOT_DIR/tests/fixtures/basic/hello.puml | $BIN -"
  "stdin_multi::cat $ROOT_DIR/tests/fixtures/structure/multi_three.puml | $BIN --multi -"
)

CSV="$OUT_DIR/latest.csv"
JSON="$OUT_DIR/latest.json"
MD="$OUT_DIR/latest.md"

echo "name,tool,mean_ms,stddev_ms,runs,timestamp_utc" > "$CSV"
echo "{" > "$JSON"
echo "  \"timestamp_utc\": \"$TS\"," >> "$JSON"
echo "  \"binary\": \"$BIN\"," >> "$JSON"
echo "  \"scenarios\": [" >> "$JSON"

printf '%s\n\n' '# Benchmark Results' > "$MD"
printf '%s\n' "- Timestamp (UTC): \`$TS\`" >> "$MD"
printf '%s\n\n' "- Binary: \`$BIN\`" >> "$MD"
printf '%s\n' '| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |' >> "$MD"
printf '%s\n' '|---|---:|---:|---:|---|' >> "$MD"

first=1
for entry in "${SCENARIOS[@]}"; do
  name="${entry%%::*}"
  cmd="${entry#*::}"

  if [[ "$HAVE_HYPERFINE" -eq 1 ]]; then
    TMP_JSON="$(mktemp)"
    hyperfine \
      --warmup 2 \
      --runs 10 \
      --export-json "$TMP_JSON" \
      "$cmd" >/dev/null

    mean_ms="$(python - << 'PY' "$TMP_JSON"
import json,sys
j=json.load(open(sys.argv[1]))
r=j['results'][0]
print(f"{r['mean']*1000:.3f}")
PY
)"
    std_ms="$(python - << 'PY' "$TMP_JSON"
import json,sys
j=json.load(open(sys.argv[1]))
r=j['results'][0]
print(f"{r['stddev']*1000:.3f}")
PY
)"
    runs="10"
    tool="hyperfine"
    rm -f "$TMP_JSON"
  else
    # Fallback: coarse timing with /usr/bin/time over 5 runs
    times=()
    for _ in 1 2 3 4 5; do
      t="$(/usr/bin/time -f '%e' bash -lc "$cmd" 2>&1 >/dev/null | tail -n1)"
      times+=("$t")
    done
    mean_ms="$(python - << 'PY' "${times[@]}"
import sys
vals=[float(x) for x in sys.argv[1:]]
mean=sum(vals)/len(vals)
print(f"{mean*1000:.3f}")
PY
)"
    std_ms="$(python - << 'PY' "${times[@]}"
import math,sys
vals=[float(x) for x in sys.argv[1:]]
m=sum(vals)/len(vals)
var=sum((x-m)**2 for x in vals)/len(vals)
print(f"{math.sqrt(var)*1000:.3f}")
PY
)"
    runs="5"
    tool="time"
  fi

  echo "$name,$tool,$mean_ms,$std_ms,$runs,$TS" >> "$CSV"
  printf '| `%s` | %s | %s | %s | `%s` |\n' "$name" "$mean_ms" "$std_ms" "$runs" "$tool" >> "$MD"

  if [[ "$first" -eq 0 ]]; then
    echo "    ," >> "$JSON"
  fi
  first=0
  cat >> "$JSON" <<J
    {
      "name": "$name",
      "tool": "$tool",
      "mean_ms": $mean_ms,
      "stddev_ms": $std_ms,
      "runs": $runs
    }
J

done

echo "  ]" >> "$JSON"
echo "}" >> "$JSON"

echo "[bench] wrote:"
echo "  - $CSV"
echo "  - $JSON"
echo "  - $MD"
