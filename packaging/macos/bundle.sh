#!/bin/sh
# SIGNING_IDENTITY and PROVISIONING_PROFILE must match Apple's approved App ID.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)

: "${SIGNING_IDENTITY:?Set a Developer ID Application signing identity}"
: "${PROVISIONING_PROFILE:?Set the path to the Apple-approved provisioning profile}"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
profile_plist="$work/profile.plist"
entitlements_plist="$work/entitlements.plist"

security cms -D -i "$PROVISIONING_PROFILE" > "$profile_plist"
team=$(/usr/libexec/PlistBuddy -c 'Print :TeamIdentifier:0' "$profile_plist")
prefix=$(/usr/libexec/PlistBuddy -c 'Print :ApplicationIdentifierPrefix:0' "$profile_plist")
allowed=$(/usr/libexec/PlistBuddy -c 'Print :Entitlements:com.apple.developer.hid.virtual.device' "$profile_plist")
if [ "$allowed" != true ]; then
    echo 'The provisioning profile does not grant virtual HID devices.' >&2
    exit 1
fi

: "${BUNDLE_ID:?Set the bundle identifier approved in the provisioning profile}"
cp "$root/packaging/macos/entitlements.plist" "$entitlements_plist"
/usr/libexec/PlistBuddy -c "Add :com.apple.application-identifier string $prefix.$BUNDLE_ID" "$entitlements_plist"
/usr/libexec/PlistBuddy -c "Add :com.apple.developer.team-identifier string $team" "$entitlements_plist"

app="$root/dist/FEIghtstick.app"
info_plist="$app/Contents/Info.plist"
mkdir -p "$app/Contents/MacOS"
cp "$root/target/release/feightstick" "$app/Contents/MacOS/feightstick"
cp "$PROVISIONING_PROFILE" "$app/Contents/embedded.provisionprofile"

/usr/libexec/PlistBuddy -c Clear "$info_plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string $BUNDLE_ID" "$info_plist"
/usr/libexec/PlistBuddy -c 'Add :CFBundleExecutable string feightstick' "$info_plist"
/usr/libexec/PlistBuddy -c 'Add :CFBundleName string FEIghtstick' "$info_plist"
/usr/libexec/PlistBuddy -c 'Add :CFBundlePackageType string APPL' "$info_plist"
/usr/libexec/PlistBuddy -c 'Add :CFBundleShortVersionString string 1.0.0' "$info_plist"

codesign --force --options runtime --timestamp --sign "$SIGNING_IDENTITY" \
    --entitlements "$entitlements_plist" "$app"
codesign --verify --strict "$app"

printf '%s\n' 'Signed bundle created in dist. Notarize and staple before public distribution.'
