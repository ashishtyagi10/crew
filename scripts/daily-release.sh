#!/bin/bash
# scripts/daily-release.sh            cut today's release, if there is one to cut
# DRY_RUN=1 scripts/daily-release.sh  write the release notes and stop
#
# Run every evening by launchd (com.atyagi.crew-daily-release). Ships what is
# merged on local main and not yet tagged, as ONE release, through
# scripts/release.sh — the same gate, version pick, tag and push a release by
# hand takes. It does nothing, and says why in the log, when:
#   - a release already went out today (one per day),
#   - nothing is merged since the newest tag,
#   - the checkout is not on main or has uncommitted edits (work in progress
#     is never shipped unattended).
#
# The only part a script cannot do is say what changed, so a headless Claude
# writes the CHANGELOG entry and the commit message from the log and the diff,
# with read-only git and nothing else. Everything that changes the repo or
# pushes is this script.
set -uo pipefail

export PATH="$HOME/.local/bin:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
cd "$(dirname "$0")/.."
OUT=target/daily-release
mkdir -p "$OUT"

say() { echo "$(date '+%Y-%m-%d %H:%M:%S') $*"; }
notify() {
  /usr/bin/osascript -e "display notification \"$1\" with title \"crew daily release\"" >/dev/null 2>&1 || true
}
stop() { say "skip: $1"; exit 0; }
fail() { say "FAILED: $1"; notify "$1"; exit 1; }

[[ $(git rev-parse --abbrev-ref HEAD) == main ]] || stop "not on main ($(git rev-parse --abbrev-ref HEAD))"
[[ -z $(git status --porcelain) ]] || stop "uncommitted edits on main"
git fetch -q --tags origin || fail "could not reach origin"

TAG=$(git tag -l 'v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname | head -1)
# (A dry run only writes notes, so it may run on a day that has its release.)
[[ -n ${DRY_RUN:-} || $(git log -1 --format=%cd --date=format:%Y-%m-%d "$TAG") != $(date +%Y-%m-%d) ]] ||
  stop "$TAG already went out today"
N=$(git rev-list --count "$TAG..main")
((N > 0)) || stop "nothing merged since $TAG"
git merge-base --is-ancestor origin/main main || fail "local main is behind or has diverged from origin/main"
say "$N commits since $TAG — writing the notes"

# What changed, said by something that can read it. Read-only git, and the
# three files it is asked for; it cannot build, commit or push.
rm -f "$OUT/ENTRY.md" "$OUT/MSG.txt" "$OUT/SUBJECT.txt"
claude -p "You are writing the notes for an unattended crew release. Do not change, build, commit or push anything.

Read what is merged since the last release: \`git log --no-merges $TAG..main\` and \`git diff --stat $TAG..main\`, and \`git diff $TAG..main -- <path>\` for anything the log leaves unclear. Read the top two entries of CHANGELOG.md for the house style.

Write exactly three files:
1. $OUT/ENTRY.md — the CHANGELOG entry BODY, with no '## version' heading (the release script adds it). One bold headline sentence saying what the release is, UNDER 70 CHARACTERS (the app's welcome card shows it and a test in the release gate fails on a longer one), then short sections of bullets written for a user of the app: what they will see, in plain words, no file names. Every claim must come from the log or the diff.
2. $OUT/MSG.txt — the release commit message. First line: 'release: <summary> ({V})' with the literal text {V} where the version goes. Then a short body. Last line: 'Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>'.
3. $OUT/SUBJECT.txt — one line under 90 characters: the bare summary, with no 'merge:' prefix and no version." \
  --allowedTools "Read" "Write" "Bash(git log:*)" "Bash(git diff:*)" "Bash(git show:*)" \
  --permission-mode acceptEdits >"$OUT/claude.log" 2>&1 || fail "the notes run failed — see $OUT/claude.log"

[[ -s $OUT/ENTRY.md && -s $OUT/MSG.txt && -s $OUT/SUBJECT.txt ]] || fail "the notes are incomplete — see $OUT/claude.log"
grep -q '{V}' "$OUT/MSG.txt" || fail "MSG.txt does not carry {V}"
! grep -q '^## ' "$OUT/ENTRY.md" || fail "ENTRY.md carries its own version heading"
SUBJ=$(head -1 "$OUT/SUBJECT.txt")
[[ -n ${DRY_RUN:-} ]] && { say "dry run — notes are in $OUT, nothing released"; exit 0; }

# release.sh ships UNCOMMITTED edits on a main that equals origin/main. So the
# merged work is kept on a branch, main steps back to origin, and the work
# comes back as one staged change. Nothing is lost if the gate fails: main is
# put back on the branch.
KEEP=rel/daily-$(date +%Y%m%d)
git branch -f "$KEEP" main
git reset -q --hard origin/main
git merge -q --squash "$KEEP" >/dev/null || { git reset -q --hard "$KEEP"; fail "could not stage the merged work"; }

# Every crate the release touches runs its own tests beside the app's.
EXTRA=$(git diff --cached --name-only | sed -n 's|^crates/\([^/]*\)/.*|\1|p' | sort -u | grep -v '^crew-app$' | tr '\n' ' ')
export EXTRA
if ! caffeinate -i scripts/release.sh "rel/v-daily-$(date +%Y%m%d)" "$OUT/ENTRY.md" "$OUT/MSG.txt" "$SUBJ"; then
  git reset -q --hard "$KEEP"
  fail "the release gate failed — main is back where it was, nothing pushed"
fi
git branch -q -D "$KEEP"
NEW=$(git describe --tags --abbrev=0)
say "released $NEW — waiting for the build"

# The tag is only half a release: the app updates from what CI publishes.
sleep 20
RUN=$(gh run list --workflow Release --limit 1 --json databaseId -q '.[0].databaseId' 2>/dev/null)
if [[ -n $RUN ]] && gh run watch "$RUN" --exit-status >/dev/null 2>&1; then
  say "$NEW is built and published"
  notify "$NEW is out — /restart in crew to apply"
else
  fail "$NEW is tagged but its Release build did not pass"
fi
