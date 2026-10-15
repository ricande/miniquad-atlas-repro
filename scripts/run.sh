#!/bin/sh
# Runs the reproduction N times (default 10) and counts the runs whose text
# turned black. Extra arguments go to the program (e.g. --rect).
set -eu
cd "$(dirname "$0")/.."
runs=${1:-10}
[ $# -gt 0 ] && shift
cargo build --release --quiet
fails=0
for i in $(seq "$runs"); do
    ./target/release/miniquad-atlas-repro 100 "$@" >/dev/null || fails=$((fails + 1))
done
echo "$fails of $runs runs drew black text"
