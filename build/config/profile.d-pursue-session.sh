#!/bin/sh
# PURSUE OS — Automatic Sway session launcher on physical/VM primary console (tty1)
if [ -z "$WAYLAND_DISPLAY" ] && [ -z "$DISPLAY" ] && [ "$(tty)" = "/dev/tty1" ]; then
    # Enable software rendering fallback and software cursors for broad GPU compatibility (NVIDIA, simpledrm, virtual)
    export WLR_NO_HARDWARE_CURSORS=1
    export WLR_RENDERER_ALLOW_SOFTWARE=1

    # Launch Sway with --unsupported-gpu to permit running on NVIDIA and non-standard DRM stacks
    sway --unsupported-gpu
    SWAY_EXIT=$?
    if [ $SWAY_EXIT -ne 0 ]; then
        echo "=========================================================="
        echo " PURSUE OS — Graphical Session Notice (Exit code: $SWAY_EXIT)"
        echo " Sway compositor could not initialize the graphical display."
        echo " The PURSUE OS Core Runtime daemon is active in the background."
        echo " You can inspect logs, run live verification, or retry Sway:"
        echo "   journalctl -u pursue-runtime.service -n 20"
        echo "   /usr/lib/pursue/bin/pursue-desktop --verify-live"
        echo "   WLR_RENDERER=pixman sway --unsupported-gpu -d"
        echo "=========================================================="
    fi
fi

