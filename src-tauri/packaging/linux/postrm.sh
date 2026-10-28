#!/bin/sh
udevadm control --reload-rules 2>/dev/null || true
exit 0
