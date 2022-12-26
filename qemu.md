# QEMU

## installation
docs: https://wiki.archlinux.org/title/QEMU

```s
sudo pacman -Sy qemu-full virt-manager dmidecode

systemctl enable libvirtd
systemctl start libvirtd

sudo usermod -a -G libvirt user
```
GUI: `virt-manager`

## network bridge
docs: https://wiki.archlinux.org/title/Systemd-networkd#Bridge_interface
      https://www.freedesktop.org/software/systemd/man/systemd.network.html

`/etc/systemd/network/qemu0.netdev`
```ini
[NetDev]
Name=qemu0
Kind=bridge
```

`/etc/systemd/network/qemu0.network`
```ini
[Match]
Name=qemu0

[Network]
Address=10.0.0.1/24
IPMasquerade=true
IPForward=true
DHCPServer=true

[DHCPServer]
PoolOffset=1
PoolSize=50
EmitDNS=yes
DNS=1.1.1.1
```

`/etc/systemd/network/qemu0-wlp166s0-uplink.network`
```ini
[Match]
Name=wlp166s0

[Network]
Bridge=br0
```

allow QEMU to use the bridge `/etc/qemu/bridge.conf`:
```s
allow qemu0
```

```s
systemctl restart systemd-networkd libvirtd
```
