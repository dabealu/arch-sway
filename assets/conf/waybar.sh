#!/usr/bin/env sh
killall -q waybar
while pgrep -x waybar >/dev/null; do sleep 1; done
exec waybar -c ~/.config/sway/waybar.json -s ~/.config/sway/waybar.css
