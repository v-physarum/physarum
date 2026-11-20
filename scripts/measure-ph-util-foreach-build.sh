#!/usr/bin/env bash
# Time a clean dev build of ph_util_foreach and a no-change incremental build.
# Downloads are fetched first and are not included in the timed commands.
# The output directory is recreated and contains its own target directory.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

out="${1:-"$root/target/ph-util-foreach-build-time"}"
rm -rf "$out"
mkdir -p "$out"
export CARGO_TARGET_DIR="$out/target"

cargo fetch --locked

time_cmd() {
    local label=$1
    local logfile=$2
    shift 2
    local start end status
    start="$(date +%s.%N)"
    "$@" >"$logfile" 2>&1
    status=$?
    end="$(date +%s.%N)"
    if [[ "$status" -ne 0 ]]; then
        cat "$logfile" >&2
        exit "$status"
    fi
    awk -v label="$label" -v start="$start" -v end="$end" \
        'BEGIN { printf "%s %.3f\n", label, (end - start) }'
}

cargo_seconds() {
    local label=$1
    local logfile=$2
    local line seconds
    line="$(grep -E 'Finished `dev` profile' "$logfile" | tail -n 1)"
    seconds="$(sed -n 's/.* in \([0-9.][0-9.]*\)s.*/\1/p' <<<"$line")"
    if [[ -z "$seconds" ]]; then
        echo "missing Finished line in $logfile" >&2
        exit 1
    fi
    printf '%s %s\n' "$label" "$seconds"
}

{
    echo "toolchain $(rustc --version)"
    echo "package ph_util_foreach"
    echo "profile dev"
    echo "clean_command cargo build -p ph_util_foreach --offline --locked --timings"
    echo "incremental_command cargo build -p ph_util_foreach --offline --locked"
    time_cmd clean_wall_seconds "$out/clean.log" \
        cargo build -p ph_util_foreach --offline --locked --timings
    cargo_seconds clean_cargo_seconds "$out/clean.log"
    time_cmd incremental_wall_seconds "$out/incremental.log" \
        cargo build -p ph_util_foreach --offline --locked
    cargo_seconds incremental_cargo_seconds "$out/incremental.log"
} >"$out/report.txt"

cp "$out/target/cargo-timings/cargo-timing.html" "$out/cargo-timing.html"

python3 - "$out/cargo-timing.html" >>"$out/report.txt" <<'PY'
import json
import re
import sys
from pathlib import Path

html = Path(sys.argv[1]).read_text()
match = re.search(r"const UNIT_DATA = (\[.*?]);", html, re.S)
if match is None:
    raise SystemExit("UNIT_DATA missing from cargo timing report")
units = json.loads(match.group(1))
print("units")
total = 0.0
for unit in sorted(units, key=lambda item: (-item["duration"], item["name"], item["target"])):
    total += unit["duration"]
    target = unit["target"].strip() or "lib"
    print(f"  {unit['duration']:.2f}s  {unit['name']} {unit['version']} {target}")
print(f"unit_duration_sum_seconds {total:.2f}")
own = [
    unit["duration"]
    for unit in units
    if unit["name"] == "ph_util_foreach" and unit["target"].strip() == ""
]
if len(own) != 1:
    raise SystemExit("expected one ph_util_foreach lib unit")
print(f"ph_util_foreach_lib_seconds {own[0]:.2f}")
PY

cat "$out/report.txt"

if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    {
        echo "## ph_util_foreach build time"
        echo
        echo '```'
        cat "$out/report.txt"
        echo '```'
    } >>"$GITHUB_STEP_SUMMARY"
fi
