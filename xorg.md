# xorg

in case app requires Xorg or works poorly on wayland.

- install xorg and LXDE:
```sh
pacman -Sy xorg xorg-xinit lxde

- copy xinitrc
```sh
cp /etc/X11/xinit/xinitrc ~/.xinitrc
```

- edit `~/.xinitrc` and at the bottom of the file comment `twm &` and lines below
- add `exec startlxde`
- start desktop environment with `startx`
- for Steam consider to use `gamescope` - https://wiki.archlinux.org/title/Gamescope
