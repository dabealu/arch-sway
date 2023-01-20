#!/bin/bash

set -e

cargo build -r

mkdir archiso
cd archiso/

cp -r /usr/share/archiso/configs/releng .
cp ../target/release/arch-sway releng/airootfs/usr/local/bin/

sed -i 's#file_permissions=(#file_permissions=(\n  ["/root/arch-sway"]="0:0:755"#' releng/profiledef.sh

sudo mkarchiso -v -w . -o ../archlinux-iso releng/

cd ..
sudo rm -rf archiso/

lsblk

echo "create installation media:"
echo "sudo cp archlinux-iso/archlinux-2022.12.26-x86_64.iso /dev/sdX"
