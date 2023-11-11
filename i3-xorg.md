# i3

in case app requires Xorg or works poorly on wayland.

install i3:
```sh
pacman -Sy xorg xorg-xinit i3 dmenu
cp /etc/X11/xinit/xinitrc ~/.xinitrc
```

edit `~/.xinitrc` and at the bottom of the file comment `twm &` and lines below,
add `exec i3`, and run i3 with `startx`.
