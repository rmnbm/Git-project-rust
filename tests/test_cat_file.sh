#!/usr/bin/env bash
#
# Step 1: Test that "cat-file -p <hash>" pretty-prints blob contents.
# Simulates: init, insert blob via git hash-object, then our program cat-file -p.
#
# Usage: ./scripts/test_step1_cat_file.sh [path/to/your_program.sh]
#        If omitted, path is $(git rev-parse --show-toplevel)/your_program.sh
#        when run from repo, or ./your_program.sh from project root.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PROGRAM="${1:-$REPO_ROOT/your_program.sh}"
CONTENT="$(hexdump -vn16 -e'4/4 "%08x" 1 "\n"' /dev/urandom)"

if [[ ! -f "$PROGRAM" ]]; then
  echo "Error: Program not found: $PROGRAM" >&2
  exit 1
fi

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "$TEST_DIR"' EXIT

cd "$TEST_DIR"

"$PROGRAM" init
echo "$CONTENT" > test.txt
HASH="$(git hash-object -w test.txt)"

OUTPUT="$("$PROGRAM" cat-file -p "$HASH")"

if [[ "$OUTPUT" != "$CONTENT" ]]; then
  echo "FAIL: output does not match blob contents."
  echo "Expected: $CONTENT"
  echo "Got:      $OUTPUT"
  exit 1
fi

echo "PASS: cat-file -p $HASH printed blob contents correctly."
