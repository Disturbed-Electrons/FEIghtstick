#!/bin/sh
# Keep the profile unless --purge is supplied.
set -eu

case "${1:-}" in
    ''|--purge) ;;
    *)
        echo 'Usage: uninstall.sh [--purge]' >&2
        exit 1
        ;;
esac

if [ "$(id -u)" -ne 0 ]; then
    echo 'Run this uninstaller with sudo.' >&2
    exit 1
fi
if [ "$(uname -s)" != Linux ] || ! command -v systemctl >/dev/null; then
    echo 'This uninstaller requires Linux with systemd.' >&2
    exit 1
fi

if [ -f /etc/systemd/system/feightstick.service ]; then
    systemctl disable --now feightstick
fi

rm -f /usr/local/bin/feightstick \
    /etc/systemd/system/feightstick.service \
    /etc/udev/rules.d/70-feightstick.rules \
    /etc/modules-load.d/feightstick.conf

systemctl daemon-reload
udevadm control --reload-rules
udevadm trigger --subsystem-match=hidraw
udevadm trigger --subsystem-match=misc --sysname-match=uinput

if id feightstick >/dev/null 2>&1; then
    userdel feightstick
fi
if getent group feightstick >/dev/null; then
    groupdel feightstick
fi

if [ "${1:-}" = --purge ]; then
    rm -f /etc/feightstick/profile.toml
    rmdir /etc/feightstick 2>/dev/null || true
fi

rm -f /usr/local/libexec/feightstick/uninstall.sh
rmdir /usr/local/libexec/feightstick 2>/dev/null || true

printf '%s\n' 'FEIghtstick removed. Profiles are preserved unless --purge was supplied.'
