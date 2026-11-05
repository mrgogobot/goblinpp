#!/bin/sh
# Exercise the installed native AI planner. No Python, downloads or training.
set -eu
if [ "$#" -ne 2 ] || [ "$1" != --goblinpp ]; then
    printf 'Usage: %s --goblinpp /path/to/goblin++\n' "$0" >&2
    exit 2
fi
root=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
bin_dir=$(CDPATH= cd "$(dirname "$2")" && pwd)
binary="$bin_dir/$(basename "$2")"
work=$(mktemp -d "${TMPDIR:-/tmp}/goblin-ai-installed.XXXXXX")
# This exact directory was newly created above; no caller-selected path is removed.
trap 'rm -r "$work"' 0
trap 'exit 1' 1 2 15
cp "$root/examples/ai_resources.json" "$root/examples/ai_resources.gbl" "$root/examples/ai_quickstart.gbl" "$work/"
cd "$work"
"$binary" ai hardware
# A preserved refusal or unknown capacity is a valid plan, not a successful fit.
"$binary" ai plan ai_resources.json --output plan || {
    result=$?
    [ "$result" -eq 1 ] || [ "$result" -eq 2 ]
}
"$binary" ai verify-plan plan
for source in ai_resources.gbl ai_quickstart.gbl; do
    "$binary" run "$source"
    "$binary" run "$source" --compile
done
[ ! -e runs ] && [ ! -e .goblin ]
if "$binary" ai plan ai_resources.json --output plan; then
    printf 'FAIL: existing directory was accepted\n' >&2
    exit 1
fi
printf '{}\n' > plan/plan.json
if "$binary" ai verify-plan plan; then
    printf 'FAIL: altered plan was accepted\n' >&2
    exit 1
fi
printf 'INSTALLED_AI_CHECKS=PASS\nTRAINING_PERFORMED=false\n'
