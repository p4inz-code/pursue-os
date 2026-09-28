#!/bin/sh
# PURSUE OS — Automatic Sway session launcher on physical/VM primary console (tty1)
if [ -z "$WAYLAND_DISPLAY" ] && [ -z "$DISPLAY" ] && [ "$(tty)" = "/dev/tty1" ]; then
    exec sway
fi
