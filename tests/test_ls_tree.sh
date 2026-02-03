#!/usr/bin/env bash
#
# Step 3: Test that "ls-tree --name-only <tree_sha>" prints tree entry names.
#
# The tester writes a tree object directly into .git/objects. This script simulates that
# by using `git hash-object -w` to write blobs and `git mktree` to write tree objects.
#
# Usage: ./scripts/test_step3_ls_tree.sh [path/to/your_program.sh]

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

# Directory structure:
# - file1
# - dir1/file_in_dir_1
# - dir1/file_in_dir_2
# - dir2/file_in_dir_3
mkdir -p dir1 dir2
echo "$(rand_hex_32)" > file1
echo "$(rand_hex_32)" > dir1/file_in_dir_1
echo "$(rand_hex_32)" > dir1/file_in_dir_2
echo "$(rand_hex_32)" > dir2/file_in_dir_3

blob_file1="$(git hash-object -w file1)"
blob_d1_1="$(git hash-object -w dir1/file_in_dir_1)"
blob_d1_2="$(git hash-object -w dir1/file_in_dir_2)"
blob_d2_3="$(git hash-object -w dir2/file_in_dir_3)"

# Build subtrees first.
tree_dir1="$(
  printf "100644 blob %s\tfile_in_dir_1\n100644 blob %s\tfile_in_dir_2\n" "$blob_d1_1" "$blob_d1_2" \
    | git mktree
)"
tree_dir2="$(
  printf "100644 blob %s\tfile_in_dir_3\n" "$blob_d2_3" \
    | git mktree
)"

# Root tree references subtrees and file1.
tree_root="$(
  printf "040000 tree %s\tdir1\n040000 tree %s\tdir2\n100644 blob %s\tfile1\n" "$tree_dir1" "$tree_dir2" "$blob_file1" \
    | git mktree
)"

EXPECTED="$(git ls-tree --name-only "$tree_root")"
OUTPUT="$("$PROGRAM" ls-tree --name-only "$tree_root")"

if [[ "$OUTPUT" != "$EXPECTED" ]]; then
  echo "FAIL: ls-tree output mismatch."
  echo "Expected:"
  printf '%s\n' "$EXPECTED"
  echo "Got:"
  printf '%s\n' "$OUTPUT"
  exit 1
fi

echo "PASS: ls-tree --name-only printed expected entries for $tree_root."
