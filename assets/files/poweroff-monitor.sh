#!/bin/bash -e

# load module
if ! lsmod | grep -q i2c_dev; then
  echo "loading module"
  sudo modprobe i2c-dev
fi

sudo ddcutil --display 1 setvcp D6 05
