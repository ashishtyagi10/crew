#!/bin/bash
# scripts/release.sh BRANCH ENTRY.md MSG.txt "merge subject"
# scripts/release.sh --next            print the version the next release takes
# scripts/release.sh --after X.Y.Z     print the version that follows X.Y.Z
#
# Ships the uncommitted edits on main as one release: creates BRANCH, puts
# ENTRY.md on top of CHANGELOG.md, bumps the workspace version, runs the
# crew-app gate (plus each crate in $EXTRA), commits MSG.txt, merges --no-ff
# into main, tags and pushes main and the tag together. CI builds the tag and
# the app picks it up through /update.
#
# The script picks the version, so no caller can: one past the newest tag on
# origin, and a minor's patch runs 0..99 — after X.Y.99 comes X.(Y+1).0. Write
# {V} in MSG.txt or the subject where the version goes.
#
# Two sessions can release from the same clone, so the version is chosen
# after a fetch, and main and the tag are checked again just before tagging.
set -euo pipefail

next_after() {
  local ma mi pa
  IFS=. read -r ma mi pa <<<"$1"
  pa=$((pa + 1))
  if ((pa >= 100)); then mi=$((mi + 1)); pa=0; fi
  echo "$ma.$mi.$pa"
}

newest_tag() {
  git tag -l 'v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname | head -1
}

cd "$(git rev-parse --show-toplevel)"

# A git step that touches the index, retried while another process holds
# .git/index.lock: the running crew app's own git watcher takes it for a
# moment, and a release once died mid-undo on exactly that.
g() {
  local i
  for i in 1 2 3 4 5 6 7 8; do
    git "$@" && return 0
    [[ -e .git/index.lock ]] || return 1
    sleep 1
  done
  return 1
}

case "${1:-}" in
--after) next_after "$2"; exit 0 ;;
--next) git fetch -q --tags origin; next_after "$(newest_tag | cut -c2-)"; exit 0 ;;
esac

BR=$1; ENTRY=$2; MSG=$3; SUBJ=$4
[[ $(git rev-parse --abbrev-ref HEAD) == main ]] || { echo "not on main"; exit 1; }
git fetch -q --tags origin
[[ $(git rev-parse main) == $(git rev-parse origin/main) ]] || {
  echo "main is not origin/main — pull first"; exit 1; }
V=$(next_after "$(newest_tag | cut -c2-)")
echo "releasing v$V"
SUBJ=${SUBJ//\{V\}/$V}
BODY=$(sed "s/{V}/$V/g" "$MSG")
stale=$(grep -o -E '\(v[0-9]+\.[0-9]+\.[0-9]+\)' <<<"$BODY" | grep -v -F "(v$V)" || true)
[[ -z $stale ]] || { echo "MSG names $stale but this release is v$V — write {V}"; exit 1; }

# The version is part of cargo's build hash, so every bump compiles each crew
# crate under a new name and the old copies, a gigabyte or more a release,
# stayed in target/ for good. They rebuild for v$V anyway, so dropping them
# here costs no build time. The guard keeps an empty list from becoming a
# bare `cargo clean` of the whole target dir.
crates=$(cargo metadata -q --no-deps --format-version 1 | /usr/bin/python3 -c \
  'import json, sys; print(" ".join("-p " + p["name"] for p in json.load(sys.stdin)["packages"]))')
[[ $crates == -p* ]] && cargo clean $crates

g checkout -q -b "$BR"
/usr/bin/python3 - "$ENTRY" "$V" <<'EOF'
import sys
entry, v = open(sys.argv[1]).read().strip() + "\n\n", sys.argv[2]
p = "CHANGELOG.md"; s = open(p).read()
i = s.index("\n## ") + 1
s = s[:i] + f"## {v}\n\n" + entry + s[i:]
open(p, "w").write(s)
EOF
sed -i '' -E "1,30s/^version = \"[0-9.]+\"/version = \"$V\"/" Cargo.toml
grep -q "^version = \"$V\"" Cargo.toml
cargo fmt --all

# A failed gate puts everything back as it was, the fix still uncommitted on
# main, so the script can simply be run again.
undo() {
  g checkout -q HEAD -- CHANGELOG.md Cargo.toml Cargo.lock
  g checkout -q main
  g branch -q -D "$BR"
  echo "GATE FAILED ($1) — back on main, fix still uncommitted"
  exit 2
}
log=$(mktemp)
cargo test -p crew-app --bin crew >"$log" 2>&1 || {
  grep -E "^test result|FAILED|panicked|^error" "$log" | head -20; undo crew-app; }
grep -E "^test result" "$log"
for p in ${EXTRA:-}; do
  cargo test -p "$p" >"$log" 2>&1 || {
    grep -E "^test result: FAILED|FAILED|panicked|^error" "$log" | head -20; undo "$p"; }
done

g add -A
git status --short
g commit -q -F - <<<"$BODY"
g checkout -q main
g merge -q --no-ff "$BR" -m "merge: $SUBJ (v$V)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"

git fetch -q --tags origin
if [[ $(git rev-parse origin/main) != $(git rev-parse main^1) ]] || git rev-parse -q --verify "refs/tags/v$V" >/dev/null; then
  echo "origin moved or v$V was taken while the gate ran — main holds an unpushed merge;"
  echo "reset main to origin/main, git checkout $BR -- <files>, and run again"
  exit 3
fi
g tag "v$V"
git push -q --atomic origin main "v$V"
g branch -q -d "$BR"
git log --oneline -3
echo "RELEASED v$V"
