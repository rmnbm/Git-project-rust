#!/usr/bin/env bash
#
# Step 4: Test that "write-tree" writes the working directory as tree objects
# and prints the same root tree SHA-1 as official git.
#
# Usage: ./scripts/test_step4_write_tree.sh [path/to/your_program.sh]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PROGRAM="${1:-$REPO_ROOT/your_program.sh}"

if [[ ! -f "$PROGRAM" ]]; then
  echo "Error: Program not found: $PROGRAM" >&2
  exit 1
fi

rand_hex_32() {
  hexdump -vn16 -e'4/4 "%08x" 1 "\n"' /dev/urandom
}

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "$TEST_DIR"' EXIT
cd "$TEST_DIR"

"$PROGRAM" init

echo "$(rand_hex_32)" > test_file_1.txt
mkdir test_dir_1
echo "$(rand_hex_32)" > test_dir_1/test_file_2.txt
mkdir test_dir_2
echo "$(rand_hex_32)" > test_dir_2/test_file_3.txt

# Official git computes write-tree from the index, so stage the working tree first.
git add -A
EXPECTED="$(git write-tree)"

OUTPUT="$("$PROGRAM" write-tree | tr -d '\n\r')"

if [[ ${#OUTPUT} -ne 40 || ! "$OUTPUT" =~ ^[0-9a-f]{40}$ ]]; then
  echo "FAIL: expected 40-character hex SHA-1, got: $OUTPUT" >&2
  exit 1
fi

if [[ "$OUTPUT" != "$EXPECTED" ]]; then
  echo "FAIL: write-tree hash mismatch."
  echo "Expected (git write-tree): $EXPECTED"
  echo "Got (your program):        $OUTPUT"
  exit 1
fi

echo "PASS: write-tree produced expected tree hash ($OUTPUT)."

