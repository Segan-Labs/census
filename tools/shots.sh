#!/usr/bin/env bash
# Regenerate docs/shots/*.png from a snapshot saved with `census --json`.
#   tools/shots.sh SNAPSHOT.json
# Uses target/release/census, or the one at $CARGO_TARGET_DIR/release/census.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
snap=$1
census="${CARGO_TARGET_DIR:-$root/target}/release/census --snapshot $snap --color always"
# The PNG renderer's cells are 11 by 26 px, so the logo is drawn for that shape.
export CENSUS_CELL_ASPECT=2.36
out="$root/docs/shots"
mkdir -p "$out"
png() { python3 "$root/tools/ansi2png.py" "$out/$1.png" "$2"; }
png_light() { python3 "$root/tools/ansi2png.py" --light "$out/$1.png" "$2"; }

$census --width 150 --height 36 | png classic-wide 150
$census --width 150 --height 36 | png_light classic-wide-light 150
$census --width 64 --height 26 | png classic-narrow 64
$census --width 96 --height 16 | png classic-short 96
$census --layout compact --width 80 --height 36 | png compact 80
$census --layout compact --width 80 --height 36 | png_light compact-light 80
