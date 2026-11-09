#!/usr/bin/env bash
# Mean wall time per run of census (warm daemon) and fastfetch, over N runs (default 100).
n=${1:-100}
bin=${CENSUS_BIN:-$(dirname "$0")/../target/release/census}
t() {
  local start end
  start=$(date +%s%N)
  for _ in $(seq "$n"); do "$@" >/dev/null 2>&1; done
  end=$(date +%s%N)
  echo "$(( (end - start) / n / 1000 )) us/run: $*"
}
t "$bin"
command -v fastfetch >/dev/null && t fastfetch
