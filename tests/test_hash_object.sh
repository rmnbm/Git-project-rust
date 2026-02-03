#!/usr/bin/env bash
#
# Step 2: Test that "hash-object -w <file>" writes a valid blob and prints its 40-char SHA-1.
# Verifies: stdout is 40 hex chars; .git/objects blob matches what official git would write.
#
# Usage: ./scripts/test_step2_hash_object.sh [path/to/your_program.sh]

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

HASH="$("$PROGRAM" hash-object -w test.txt | tr -d '\n\r')"

if [[ ${#HASH} -ne 40 ]]; then
  echo "FAIL: expected 40-character hash, got ${#HASH} characters: $HASH" >&2
  exit 1
fi

if [[ ! "$HASH" =~ ^[0-9a-f]{40}$ ]]; then
  echo "FAIL: hash must be 40 hex digits, got: $HASH" >&2
  exit 1
fi

GIT_CONTENT="$(git cat-file -p "$HASH")"
if [[ "$GIT_CONTENT" != "$CONTENT" ]]; then
  echo "FAIL: .git/objects blob does not match what official git would write."
  echo "Expected content (via git cat-file -p): $CONTENT"
  echo "Got: $GIT_CONTENT"
  exit 1
fi

echo "PASS: hash-object -w printed 40-char SHA-1 and wrote matching blob ($HASH)."
