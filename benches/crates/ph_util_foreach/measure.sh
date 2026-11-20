#!/usr/bin/env bash
# Time impl_foreach! in a downstream crate.
# The proc-macro is compiled once and is not part of the case times.
# Debug and release builds are full compiles. Incremental time is debug only.
# Runtime time is the release bench.

set -euo pipefail

# CI disables incremental compilation and debuginfo and forces colored logs.
# Leave incremental off so a cleaned user crate is a full rustc run, and keep
# the default debuginfo of a dev build.
unset CARGO_PROFILE_DEV_DEBUG
export CARGO_INCREMENTAL=0
export CARGO_TERM_COLOR=never

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$root"

out="${1:-"$root/target/ph-util-foreach-expand"}"
rm -rf "$out"
mkdir -p "$out/user/src" "$out/logs"
export CARGO_TARGET_DIR="$out/target"

macro_dir="$root/crates/util/ph_util_foreach"
user="$out/user"

cat >"$user/Cargo.toml" <<EOF
[workspace]

[package]
name = "ph_util_foreach_user"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
ph_util_foreach = { path = "$macro_dir" }
EOF

# Writes one user lib.rs. The case name selects the invocation shape.
write_source() {
    local case=$1
    local dest=$2
    python3 - "$case" "$dest" <<'PY'
import sys
from pathlib import Path

case, dest = sys.argv[1], sys.argv[2]
kind, _, count = case.rpartition("_")
n = int(count)
lines = ["use ph_util_foreach::impl_foreach;", ""]

if kind == "flat_structs":
    lines += [
        f"impl_foreach!(range = 0..{n}, body = {{",
        "    pub struct Ix + #;",
        "});",
    ]
elif kind == "hand_structs":
    lines = ["#![allow(unused_imports)]", *lines]
    lines += [f"pub struct Ix{i};" for i in range(n)]
elif kind == "one_call":
    lines += [
        f"impl_foreach!(range = 0..{n}, body = {{",
        "    pub struct Ix + #;",
        "});",
    ]
elif kind == "many_calls":
    for i in range(n):
        lines += [
            f"impl_foreach!(range = {i}..{i + 1}, body = {{",
            "    pub struct Ix + #;",
            "});",
        ]
elif kind == "fan_take":
    lines += [
        "macro_rules! impl_take {",
        "    (",
        "        args: ($($args:ident),* $(,)?),",
        "        ix: $ix:ident,",
        "        out: $out:ident,",
        "        rest: $($rest:ident),* $(,)?",
        "    ) => {",
        "        #[allow(non_snake_case)]",
        "        mod $ix {",
        "            pub const ARGS: &[&str] = &[$(stringify!($args)),*];",
        "            pub const OUT: &str = stringify!($out);",
        "            pub const REST: &[&str] = &[$(stringify!($rest)),*];",
        "        }",
        "    };",
        "}",
        "",
        f"impl_foreach!(range = 0..{n}, body = {{",
        "    impl_take!(",
        "        args: (T[..=#]),",
        "        ix: Ix + #,",
        "        out: T + #,",
        "        rest: T[..#] + Off[#] + T[#+1..=#],",
        "    );",
        "});",
    ]
elif kind == "fan_rows":
    lines += [
        f"impl_foreach!(range = 0..{n}, body = {{",
        "    pub const Row + #: &str = stringify!((T + #, U + #)[0..=#]);",
        "});",
    ]
else:
    raise SystemExit(f"unknown case {case}")

Path(dest).write_text("\n".join(lines) + "\n")
PY
}

cargo_seconds() {
    local logfile=$1
    local line seconds
    line="$(grep -E 'Finished' "$logfile" | tail -n 1 || true)"
    line="$(printf '%s\n' "$line" | sed -E 's/\x1b\[[0-9;]*m//g')"
    seconds="$(sed -n 's/.* in \([0-9.][0-9.]*\)s.*/\1/p' <<<"$line")"
    if [[ -z "$seconds" ]]; then
        echo "missing Finished line in $logfile" >&2
        exit 1
    fi
    printf '%s\n' "$seconds"
}

median() {
    sort -n | awk '
        { values[NR] = $1 }
        END {
            mid = int((NR + 1) / 2)
            if (NR % 2 == 1) {
                printf "%.3f\n", values[mid]
            } else {
                printf "%.3f\n", (values[mid] + values[mid + 1]) / 2
            }
        }
    '
}

# Compile the proc-macro before any timed case.
write_source flat_structs_1 "$user/src/lib.rs"
(
    cd "$user"
    cargo build --offline
) >"$out/logs/warmup.log" 2>&1

cases=(
    flat_structs_64
    flat_structs_1024
    flat_structs_8192
    hand_structs_64
    hand_structs_1024
    hand_structs_8192
    one_call_256
    many_calls_256
    fan_take_16
    fan_take_64
    fan_rows_64
    fan_rows_256
)

repeats=3
report="$out/report.txt"
{
    echo "toolchain $(rustc --version)"
    echo "package ph_util_foreach_user"
    echo "proc_macro compiled once before these cases"
    echo "repeats $repeats"
    echo "section=debug_full incremental_off"
    echo "columns case median_cargo_seconds median_wall_seconds"
} >"$report"

for case in "${cases[@]}"; do
    write_source "$case" "$user/src/lib.rs"
    walls="$out/logs/$case.wall"
    cargos="$out/logs/$case.cargo"
    : >"$walls"
    : >"$cargos"
    for run in $(seq 1 "$repeats"); do
        (
            cd "$user"
            cargo clean -p ph_util_foreach_user
        ) >"$out/logs/$case.clean$run.log" 2>&1
        start="$(date +%s.%N)"
        set +e
        (
            cd "$user"
            cargo build --offline
        ) >"$out/logs/$case.run$run.log" 2>&1
        status=$?
        set -e
        end="$(date +%s.%N)"
        if [[ "$status" -ne 0 ]]; then
            echo "case $case run $run failed ($status)" >&2
            cat "$out/logs/$case.run$run.log" >&2
            exit "$status"
        fi
        awk -v start="$start" -v end="$end" 'BEGIN { printf "%.3f\n", (end - start) }' >>"$walls"
        cargo_seconds "$out/logs/$case.run$run.log" >>"$cargos"
    done
    cargo_median="$(median <"$cargos")"
    wall_median="$(median <"$walls")"
    printf '%s %s %s\n' "$case" "$cargo_median" "$wall_median" >>"$report"
    printf 'debug_full %s cargo %ss wall %ss\n' "$case" "$cargo_median" "$wall_median"
done

# Incremental builds are debug only. The second build does not change source.
export CARGO_INCREMENTAL=1
write_source flat_structs_1024 "$user/src/lib.rs"
(
    cd "$user"
    cargo build --offline
) >"$out/logs/debug_incremental_warmup.log" 2>&1
incremental_walls="$out/logs/debug_incremental.wall"
incremental_cargos="$out/logs/debug_incremental.cargo"
: >"$incremental_walls"
: >"$incremental_cargos"
for run in $(seq 1 "$repeats"); do
    start="$(date +%s.%N)"
    set +e
    (
        cd "$user"
        cargo build --offline
    ) >"$out/logs/debug_incremental.run$run.log" 2>&1
    status=$?
    set -e
    end="$(date +%s.%N)"
    if [[ "$status" -ne 0 ]]; then
        echo "debug incremental run $run failed ($status)" >&2
        cat "$out/logs/debug_incremental.run$run.log" >&2
        exit "$status"
    fi
    awk -v start="$start" -v end="$end" 'BEGIN { printf "%.3f\n", (end - start) }' >>"$incremental_walls"
    cargo_seconds "$out/logs/debug_incremental.run$run.log" >>"$incremental_cargos"
done
incremental_cargo="$(median <"$incremental_cargos")"
incremental_wall="$(median <"$incremental_walls")"
printf 'debug_incremental_flat_structs_1024 %s %s\n' "$incremental_cargo" "$incremental_wall" >>"$report"
printf 'debug_incremental flat_structs_1024 cargo %ss wall %ss\n' "$incremental_cargo" "$incremental_wall"

# Release build time. Incremental release builds are not measured.
export CARGO_INCREMENTAL=0
release_cases=(
    flat_structs_1024
    hand_structs_1024
    flat_structs_8192
    fan_take_64
    fan_rows_256
)
for case in "${release_cases[@]}"; do
    write_source "$case" "$user/src/lib.rs"
    walls="$out/logs/release_$case.wall"
    cargos="$out/logs/release_$case.cargo"
    : >"$walls"
    : >"$cargos"
    for run in $(seq 1 "$repeats"); do
        (
            cd "$user"
            cargo clean -p ph_util_foreach_user --release
        ) >"$out/logs/release_$case.clean$run.log" 2>&1
        start="$(date +%s.%N)"
        set +e
        (
            cd "$user"
            cargo build --release --offline
        ) >"$out/logs/release_$case.run$run.log" 2>&1
        status=$?
        set -e
        end="$(date +%s.%N)"
        if [[ "$status" -ne 0 ]]; then
            echo "release $case run $run failed ($status)" >&2
            cat "$out/logs/release_$case.run$run.log" >&2
            exit "$status"
        fi
        awk -v start="$start" -v end="$end" 'BEGIN { printf "%.3f\n", (end - start) }' >>"$walls"
        cargo_seconds "$out/logs/release_$case.run$run.log" >>"$cargos"
    done
    cargo_median="$(median <"$cargos")"
    wall_median="$(median <"$walls")"
    printf 'release_full_%s %s %s\n' "$case" "$cargo_median" "$wall_median" >>"$report"
    printf 'release_full %s cargo %ss wall %ss\n' "$case" "$cargo_median" "$wall_median"
done

# Runtime performance is release only.
runtime_target="$out/runtime-target"
runtime_log="$out/logs/runtime_build.log"
set +e
CARGO_TARGET_DIR="$runtime_target" cargo build --release --bench ph_util_foreach_runtime --offline --message-format=json >"$runtime_log" 2>"$out/logs/runtime_build.err"
status=$?
set -e
if [[ "$status" -ne 0 ]]; then
    echo "release runtime bench failed to build ($status)" >&2
    cat "$out/logs/runtime_build.err" >&2
    exit "$status"
fi
runtime_bin="$(python3 - "$runtime_log" <<'PY'
import json
import sys
from pathlib import Path
executable = None
for line in Path(sys.argv[1]).read_text().splitlines():
    if not line.startswith("{"):
        continue
    message = json.loads(line)
    if message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == "ph_util_foreach_runtime":
        executable = message.get("executable")
if not executable:
    raise SystemExit("runtime bench executable missing")
print(executable)
PY
)"
runtime_samples="$out/logs/runtime.samples"
: >"$runtime_samples"
for run in $(seq 1 "$repeats"); do
    "$runtime_bin" >"$out/logs/runtime.run$run.log"
    sed -n 's/^runtime_seconds //p' "$out/logs/runtime.run$run.log" >>"$runtime_samples"
done
runtime_median="$(awk '{ values[NR] = $1 } END { mid = int((NR + 1) / 2); if (NR % 2 == 1) printf "%.6f\n", values[mid]; else printf "%.6f\n", (values[mid] + values[mid + 1]) / 2 }' < <(sort -n "$runtime_samples"))"
printf 'runtime_release %s\n' "$runtime_median" >>"$report"
printf 'runtime_release %ss\n' "$runtime_median"

cat "$report"

if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    {
        echo "## ph_util_foreach user compile time"
        echo
        echo "Proc-macro already built. Median of $repeats runs: full debug, incremental debug, full release, and release runtime."
        echo
        echo "| case | cargo Finished (s) | wall (s) |"
        echo "| --- | ---: | ---: |"
        awk 'NF == 3 { printf "| `%s` | %s | %s |\n", $1, $2, $3 }' "$report"
    } >>"$GITHUB_STEP_SUMMARY"
fi
