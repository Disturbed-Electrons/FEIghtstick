#!/bin/sh
# Run after building: sudo sh packaging/linux/install.sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)

if [ "$(id -u)" -ne 0 ]; then
    echo 'Run this installer with sudo.' >&2
    exit 1
fi
if [ "$(uname -s)" != Linux ] || ! command -v systemctl >/dev/null; then
    echo 'This installer requires Linux with systemd.' >&2
    exit 1
fi

binary="$root/feightstick"
if [ ! -f "$binary" ]; then
    binary="$root/target/release/feightstick"
fi
if [ ! -f "$binary" ]; then
    echo 'Build first: cargo build --release' >&2
    exit 1
fi

profile=/etc/feightstick/profile.toml
if [ ! -f "$profile" ]; then
    profile="$root/profiles/default.toml"
fi
"$binary" check-profile "$profile"

if [ -f /etc/systemd/system/feightstick.service ]; then
    systemctl stop feightstick
fi

getent group feightstick >/dev/null || groupadd --system feightstick
id feightstick >/dev/null 2>&1 || useradd --system --gid feightstick \
    --no-create-home --shell /usr/sbin/nologin feightstick

install -m 755 "$binary" /usr/local/bin/feightstick
install -d /usr/local/libexec/feightstick
install -m 755 "$root/packaging/linux/uninstall.sh" /usr/local/libexec/feightstick/uninstall.sh
install -d /etc/feightstick
if [ ! -f /etc/feightstick/profile.toml ]; then
    install -m 644 "$root/profiles/default.toml" /etc/feightstick/profile.toml
fi

install -m 644 "$root/packaging/linux/70-feightstick.rules" /etc/udev/rules.d/70-feightstick.rules
install -m 644 "$root/packaging/linux/feightstick.service" /etc/systemd/system/feightstick.service
install -d /etc/modules-load.d
printf '%s\n' uinput > /etc/modules-load.d/feightstick.conf
modprobe uinput

udevadm control --reload-rules
udevadm trigger --subsystem-match=hidraw
udevadm trigger --subsystem-match=misc --sysname-match=uinput
udevadm settle

systemctl daemon-reload
systemctl enable feightstick
systemctl restart feightstick

printf '%s\n' 'Installed and running. FEIghtstick starts automatically at boot. Keep the joystick released when connecting.'
