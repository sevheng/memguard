#!/bin/sh
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload >/dev/null 2>&1 || true
    systemctl enable memguard-system-tune.service >/dev/null 2>&1 || true
    if [ -f /var/lib/memguard-system-tune/done ]; then
        systemctl enable --now ananicy-cpp.service >/dev/null 2>&1 || true
    fi
fi
