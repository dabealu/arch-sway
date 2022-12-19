#!/bin/bash -e

# load module
if ! lsmod | grep -q i2c_dev; then
    echo "loading module"
    sudo modprobe i2c-dev
fi

DISPLAY_NUM=1

function set_brightness() {
    sudo ddcutil --display $DISPLAY_NUM setvcp 10 $1 >/dev/null
}

function get_brightness() {
    sudo ddcutil --display $DISPLAY_NUM getvcp 10 | grep 'Brightness'
}

OPT=""

if [[ ! -z "$1" ]]; then
  OPT="$1"
else
  echo "select option:
  0 - very low
  1 - low
  2 - medium
  3 - high
  4 - get current value"
  read OPT
fi

# display number ranges from 1 to number of connected displays
case $OPT in
  set_brightness 10
  ;;
1)
  set_brightness 35
  ;;
2)
  set_brightness 60
  ;;
3)
  set_brightness 100
  ;;
4)
  get_brightness
  ;;
*)
  echo "invalid option ${OPT}, exiting..."
  exit 1
  ;;
esac
