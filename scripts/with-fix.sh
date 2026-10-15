#!/bin/sh
# Like run.sh, but against miniquad 0.4.11 with the fix in
# miniquad-forget-deleted-texture.patch applied: `delete_texture` makes the
# texture-binding cache forget the deleted texture. The patched copy is made
# in ./patched (ignored by git); the original crate is not touched.
set -eu
cd "$(dirname "$0")/.."
runs=${1:-10}
manifest=$(cargo metadata --format-version 1 \
    | grep -o '"manifest_path":"[^"]*miniquad-0\.4\.11/Cargo\.toml"' | cut -d'"' -f4)
rm -rf patched && mkdir -p patched
cp -r "$(dirname "$manifest")" patched/miniquad
patch -s -d patched/miniquad -p1 < miniquad-forget-deleted-texture.patch
cargo build --release --quiet --target-dir target/patched \
    --config 'patch.crates-io.miniquad.path="patched/miniquad"'
fails=0
for i in $(seq "$runs"); do
    ./target/patched/release/miniquad-atlas-repro 100 >/dev/null || fails=$((fails + 1))
done
echo "with the fix: $fails of $runs runs drew black text"
