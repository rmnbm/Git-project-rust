#!/usr/bin/env bash
#
# Step 5: Test that "commit-tree <tree_sha> -p <parent_sha> -m <message>" 
# creates a commit object and prints its SHA-1.
#
# Usage: ./scripts/test_step5_commit_tree.sh [path/to/your_program.sh]

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

# Create a simple file and tree
echo "$(rand_hex_32)" > test.txt
git add test.txt
TREE_SHA="$(git write-tree)"

# Create initial commit (no parent)
git commit-tree "$TREE_SHA" -m "Initial commit" > /dev/null 2>&1 || true
# Actually, let's use git commit to create the first commit properly
git config user.name "Test User"
git config user.email "test@example.com"
PARENT_SHA="$(echo "Initial commit" | git commit-tree "$TREE_SHA")"

# Create a second commit with our program
MESSAGE="$(rand_hex_32)"
OUTPUT="$("$PROGRAM" commit-tree "$TREE_SHA" -p "$PARENT_SHA" -m "$MESSAGE" | tr -d '\n\r')"

if [[ ${#OUTPUT} -ne 40 || ! "$OUTPUT" =~ ^[0-9a-f]{40}$ ]]; then
  echo "FAIL: expected 40-character hex SHA-1, got: $OUTPUT" >&2
  exit 1
fi

# Verify the commit object can be read by git
GIT_MESSAGE="$(git show -s --format=%B "$OUTPUT" 2>/dev/null | head -1 || echo "")"
if [[ "$GIT_MESSAGE" != "$MESSAGE" ]]; then
  echo "FAIL: commit message mismatch."
  echo "Expected: $MESSAGE"
  echo "Got (via git show): $GIT_MESSAGE"
  exit 1
fi

# Verify it has the correct parent
GIT_PARENT="$(git show -s --format=%P "$OUTPUT" 2>/dev/null || echo "")"
if [[ "$GIT_PARENT" != "$PARENT_SHA" ]]; then
  echo "FAIL: commit parent mismatch."
  echo "Expected: $PARENT_SHA"
  echo "Got (via git show): $GIT_PARENT"
  exit 1
fi

echo "PASS: commit-tree created valid commit object ($OUTPUT)."
