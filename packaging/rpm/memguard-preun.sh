#!/bin/sh
if command -v systemctl >/dev/null 2>&1; then
    systemctl stop memguard.service >/dev/null 2>&1 || true
    systemctl disable memguard.service >/dev/null 2>&1 || true
    systemctl daemon-reload >/dev/null 2>&1 || true
fi
