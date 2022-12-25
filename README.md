# sway on arch
arch + sway installation.

to start the installation:
- connect to wifi and install git:
```s
ip link set wlan0 up
wpa_supplicant -B -i wlan0 -c <(wpa_passphrase "WIFI_SSID" "$WIFI_PASSWORD")
dhcpcd
```
- install and run `wavemon` to scan wifi networks if needed
- run `./arch-sway` and follow the instructions

`tmux` can help to keep several terminals open for debugging:
`CTRL+b` and `SHIFT+"` or `SIFT+%` to split, `CTRL+b` and `up|down` to change focus.

## notes
### flashing black screen during installation
laptop may enter into loop with flashing black screen after selecting install from boot menu.
select `install`, but press `e` instead of `enter` to edit kernel parameters, add `nomodeset` parameter:
```s
linux /boot/vmlinuz-linux ... nomodeset 
initrd ...
```
press `ctrl+x` to save and load.

ref: https://wiki.archlinux.org/title/Kernel_parameters

### disable mitigations
this may increase CPU performance, but **potentially dangerous**.
disable hardware vulnerability mitigations by setting `mitigations=off` kernel parameter.

### gpm
`gpm` may help to navigate and copypaste during tty stages:
```s
pacman -S gpm
gpm -m /dev/input/mice -t imps2 # usb mouse
gpm -m /dev/input/mice -t exps2 # touchpad
gpm -m /dev/psaux -t ps2        # ps/2 mouse
```

### pipewire
https://wiki.archlinux.org/title/PipeWire
set flag to enable WebRTC in chrome: `chrome://flags/#enable-webrtc-pipewire-capturer`

### bluetooth pairing
```s
$ bluetoothctl
agent KeyboardOnly
default-agent
power on
scan on
pair 00:12:34:56:78:90
connect 00:12:34:56:78:90
```
ref: https://wiki.archlinux.org/title/bluetooth#Pairing

### screen resolution
run `swaymsg -t get_outputs` to get list of outputs and `man sway-output` for more options.
use `wdisplays` for GUI configuration.

### set external display brightness
```s
brightness.sh
```

### appearance
use `lxappearance` to set GTK theme and appearance settings.
lxappearance stores config in `~/.gtkrc-2.0`.
more themes: https://wiki.archlinux.org/title/GTK#Themes

### connecting android devices via USB
based on: https://wiki.archlinux.org/title/Media_Transfer_Protocol

install dependencies:
```s
sudo pacman -Sy android-udev android-file-transfer
```
restart may be needed.

connect phone, select `File Transfer` (MTP), keep screen unlocked.
mount phone storage:
```s
mkdir -p ~/mnt
aft-mtp-mount ~/mnt
```

### keybindings
use `wev` to get key code
```s
yay -Sy wev
```

### zoom
use web app, works well with chrome.
in case zoom app is crashing when joining the meeting:
```s
ANOM_ABEND auid=1000 uid=1000 gid=1000 ses=1 pid=4324 comm="QSGRenderThread" exe="/opt/zoom/zoom" sig=11 res=1
```
try to set env var for Qt:
```s
QSG_RENDER_LOOP=basic zoom
```

### encrypted volume
TODO
