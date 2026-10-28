#!/bin/sh
modprobe uinput 2>/dev/null || true
udevadm control --reload-rules 2>/dev/null || true
udevadm trigger --subsystem-match=misc --subsystem-match=input --action=change 2>/dev/null || true
exit 0
