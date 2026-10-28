#!/bin/sh

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)

if [ -d "$root/FEIghtstick.app" ]; then
    binary="$root/FEIghtstick.app/Contents/MacOS/feightstick"
elif [ -x "$root/feightstick" ]; then
    binary="$root/feightstick"
else
    binary="$root/target/release/feightstick"
fi

"$binary" run --profile "$root/profiles/default.toml"
status=$?
if [ "$status" -ne 0 ]; then
    printf '\nPress Enter to close. '
    read -r unused
fi
exit "$status"
