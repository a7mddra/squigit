#!/bin/sh
set -eu

if [ "$(id -u)" -ne 0 ]; then
  printf '%s\n' "Run this installer with sudo to register Squigit's native authentication policy." >&2
  exit 1
fi

policy_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
install -o root -g root -m 0644 "$policy_dir/org.squigit.policy" /usr/share/polkit-1/actions/org.squigit.policy
