#!/usr/bin/env bash
# benchmarks comparing resharp vs ripgrep
# requires: hyperfine, rg, resharp
set -euo pipefail

RESHARP="${RESHARP:-./target/release/resharp}"
RG="${RG:-rg}"
HAYSTACK_DIR="/home/ian/f/myrepos/resharp/data/haystacks"
HAYSTACK_LARGE="$HAYSTACK_DIR/rust-src-tools-3b0d4813.txt"
HAYSTACK_MEDIUM="$HAYSTACK_DIR/en-sampled-seeded.txt"

if [ ! -f "$HAYSTACK_LARGE" ] || [ ! -f "$HAYSTACK_MEDIUM" ]; then
  echo "error: haystack files not found at $HAYSTACK_DIR"
  echo "expected: rust-src-tools-3b0d4813.txt, en-sampled-seeded.txt"
  exit 1
fi

if ! command -v hyperfine &>/dev/null; then
  echo "error: hyperfine not found. install with: nix shell nixpkgs#hyperfine"
  exit 1
fi

RUNS="${RUNS:-10}"
WARMUP="${WARMUP:-3}"
HF="hyperfine --warmup $WARMUP --runs $RUNS"

echo "============================================"
echo "  resharp vs ripgrep benchmarks"
echo "============================================"
echo "  resharp: $($RESHARP --version 2>&1 | head -1)"
echo "  ripgrep: $($RG --version | head -1)"
echo "  large haystack:  $(wc -c < "$HAYSTACK_LARGE") bytes"
echo "  medium haystack: $(wc -c < "$HAYSTACK_MEDIUM") bytes"
echo "  runs: $RUNS, warmup: $WARMUP"
echo "============================================"
echo ""

echo "--- 1. literal search (large file) ---"
$HF \
  -n resharp "$RESHARP -c 'impl' '$HAYSTACK_LARGE' --color never --no-heading" \
  -n ripgrep "$RG -c 'impl' '$HAYSTACK_LARGE'"
echo ""

echo "--- 2. simple regex (large file) ---"
$HF \
  -n resharp "$RESHARP -c 'fn\s+\w+' '$HAYSTACK_LARGE' --color never --no-heading" \
  -n ripgrep "$RG -c 'fn\s+\w+' '$HAYSTACK_LARGE'"
echo ""

echo "--- 3. case insensitive (medium file) ---"
$HF \
  -n resharp "$RESHARP -ic 'the' '$HAYSTACK_MEDIUM' --color never --no-heading" \
  -n ripgrep "$RG -ic 'the' '$HAYSTACK_MEDIUM'"
echo ""

echo "--- 4. alternation (medium file) ---"
$HF \
  -n resharp "$RESHARP -c 'Sherlock|Watson|Holmes|Moriarty|Lestrade' '$HAYSTACK_MEDIUM' --color never --no-heading" \
  -n ripgrep "$RG -c 'Sherlock|Watson|Holmes|Moriarty|Lestrade' '$HAYSTACK_MEDIUM'"
echo ""

echo "--- 5. word boundary (large file) ---"
$HF \
  -n resharp "$RESHARP -wc 'self' '$HAYSTACK_LARGE' --color never --no-heading" \
  -n ripgrep "$RG -wc 'self' '$HAYSTACK_LARGE'"
echo ""

echo "--- 6. no match (large file) ---"
$HF \
  -n resharp "$RESHARP -c 'ZZZZYYYXXX_NOMATCH' '$HAYSTACK_LARGE' --color never --no-heading 2>/dev/null || true" \
  -n ripgrep "$RG -c 'ZZZZYYYXXX_NOMATCH' '$HAYSTACK_LARGE' || true"
echo ""

echo "--- 7. resharp-only: intersection (medium file) ---"
echo "    pattern: (_*the_*)&(_*and_*) — lines containing both 'the' and 'and'"
$HF \
  -n resharp "$RESHARP -c '(_*the_*)&(_*and_*)' '$HAYSTACK_MEDIUM' --color never --no-heading"
echo "(ripgrep cannot express this in a single pattern)"
echo ""

echo "--- 8. resharp-only: complement (medium file) ---"
echo "    pattern: lines containing 'the' but NOT 'and' (using -v for rg)"
$HF \
  -n resharp "$RESHARP -c '(_*the_*)&~(_*and_*)' '$HAYSTACK_MEDIUM' --color never --no-heading" \
  -n "ripgrep(2-pass)" "($RG 'the' '$HAYSTACK_MEDIUM' | $RG -vc 'and')"
echo ""

echo "--- 9. directory walk (resharp source tree) ---"
$HF \
  -n resharp "$RESHARP -c 'fn' src/ --color never --no-heading" \
  -n ripgrep "$RG -c 'fn' src/"
echo ""

echo "--- 10. inverted match (medium file) ---"
$HF \
  -n resharp "$RESHARP -vc 'the' '$HAYSTACK_MEDIUM' --color never --no-heading" \
  -n ripgrep "$RG -vc 'the' '$HAYSTACK_MEDIUM'"
echo ""

echo "============================================"
echo "  benchmarks complete"
echo "============================================"
