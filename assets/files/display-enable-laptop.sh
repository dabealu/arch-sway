#!/bin/bash
set -e

swaymsg -t command output eDP-1 enable
swaymsg -t command output eDP-1 scale 1.3

swaymsg -t command output DP-4 res 3840x2160@60Hz pos 0 0
swaymsg -t command output DP-4 scale 1.3

# swaymsg -t command font "pango:Noto Sans" 18
# swaymsg -t command reload
