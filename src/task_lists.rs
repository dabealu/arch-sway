use crate::parameters::Parameters;
use crate::tasks::*;

pub fn installation_list(parameters: Parameters) -> TaskRunner {
    // TODO: fix Parameters lifetime, use reference instead clone
    let mut r = TaskRunner::new();

    let username = &parameters.username;

    //------------------//
    // Stage 1: chroot  //
    //------------------//
    r.add(RequireUser::new("chroot", "root"));
    r.add(WifiConnect::new(parameters.clone()));
    r.add(Command::new(
        "install_git",
        "pacman -Sy --noconfirm git",
        false,
        false,
    ));
    r.add(GitRepo::new());
    r.add(Partitions::new(parameters.clone()));
    r.add(FS::new(parameters.clone()));
    r.add(Command::new(
        "update_archlinux_keyring",
        "pacman -Sy --noconfirm archlinux-keyring",
        false,
        false,
    ));
    r.add(Command::new(
        "pacstrap_packages",
        "pacstrap /mnt \
            linux linux-firmware base base-devel \
            grub efibootmgr dosfstools os-prober mtools \
            systemd-resolvconf wpa_supplicant netplan \
            openssh dnsutils curl git unzip vim sudo man man-pages tmux \
            sysstat bash-completion go",
        false,
        false,
    ));
    r.add(Command::new(
        "save_fstab",
        "genfstab -U /mnt >> /mnt/etc/fstab",
        false,
        true,
    ));
    // tasks within arch-chroot
    r.add(Command::new(
        "set_timezone",
        &format!(
            "arch-chroot /mnt ln -sf /usr/share/zoneinfo/{} /etc/localtime && \
            arch-chroot /mnt hwclock --systohc",
            &parameters.timezone
        ),
        false,
        true,
    ));
    r.add(Locales::new());
    r.add(Hostname::new(parameters.clone()));
    r.add(User::new(parameters.clone()));
    r.add(Grub::new(parameters.clone()));
    r.add(Info::new("reboot and continue installation as root"));
    r.add(StageCompleted::new(
        "chroot_stage_completed",
        "/mnt",
        "root",
    ));

    //------------------//
    // Stage 2: install //
    //------------------//
    r.add(RequireUser::new("install", "root"));
    r.add(Command::new(
        "enable_ntp",
        "timedatectl set-ntp true",
        false,
        false,
    ));
    r.add(Network::new(parameters.clone()));
    r.add(Resolved::new());
    r.add(Netplan::new(parameters.clone()));
    // TODO: create pacman struct - check installed packages and install only diff
    r.add(Command::new(
        "install_sway_packages",
        "pacman -Sy --noconfirm \
            sway swaylock swayidle waybar light xorg-xwayland \
            bemenu-wayland libnotify dunst wl-clipboard alacritty",
        false,
        false,
    ));
    r.add(Info::new("base desktop installed"));
    r.add(Command::new(
        "install_utilities_fonts_themes",
        "pacman -Sy --noconfirm \
            grim slurp ddcutil lxappearance \
            syslinux lshw pciutils usbutils \
            noto-fonts noto-fonts-cjk noto-fonts-emoji \
            materia-gtk-theme papirus-icon-theme adwaita-qt5",
        false,
        false,
    ));
    r.add(Variables::new());
    r.add(SwayConfigs::new(parameters.clone()));
    r.add(Command::new(
        "install_pipewire",
        "pacman -Sy --noconfirm \
            pipewire pipewire-pulse wireplumber \
            gst-plugin-pipewire xdg-desktop-portal-wlr",
        false,
        false,
    ));
    r.add(Swap::new());
    r.add(Hibernation::new());
    r.add(TextFile::new(
        "/etc/sysctl.d/01-swappiness.conf",
        "vm.swappiness = 1",
    ));
    r.add(CpuGovernor::new());
    r.add(Bluetooth::new());
    r.add(Docker::new(parameters.clone()));
    r.add(Command::new(
        "install_rust_toolchain",
        &format!(
            "pacman -Sy --noconfirm rustup && \
            sudo -u {username} -- rustup default stable"
        ),
        false,
        true,
    ));
    r.add(Command::new(
        "install_yay_aur",
        &format!(
            "sudo -u {username} -- bash -c 'mkdir -p ~/src && cd ~/src && \
            git clone https://aur.archlinux.org/yay-git.git && \
            cd yay-git && \
            makepkg --noconfirm -si'"
        ),
        false,
        true,
    ));
    r.add(Command::new(
        "install_aur_packages",
        &format!(
            "sudo -u {username} -- bash -c 'yes | yay --noconfirm -Sy \
                google-chrome \
                wdisplays \
                libinput-gestures'"
        ),
        false,
        true,
    ));
    r.add(Command::new(
        "add_user_to_input_group",
        &format!("usermod -aG input {username}"),
        false,
        false,
    ));
    r.add(Command::new(
        "start_pipewire",
        &format!(
            "systemctl --user -M {username}@.host enable pipewire pipewire-pulse && \
            systemctl --user -M {username}@.host start pipewire pipewire-pulse"
        ),
        false,
        true,
    ));
    r.add(Bashrc::new(parameters.clone()));
    r.add(Command::new(
        "install_desktop_apps",
        "sudo pacman -Sy --noconfirm \
            code \
            evince \
            xournalpp \
            telegram-desktop \
            ristretto \
            drawing \
            transmission-gtk \
            vlc \
            pavucontrol \
            thunar",
        false,
        false,
    ));
    r.add(Info::new("installation finished: reboot and run `sway`"));
    r.add(StageCompleted::new("installation_completed", "", &username));

    r
}

pub fn sync_list(parameters: Parameters) -> TaskRunner {
    let mut r = TaskRunner::new();

    r.add(RequireUser::new("config_sync", "root"));
    r.add(GitRepo::new());
    r.add(Variables::new());
    r.add(SwayConfigs::new(parameters.clone()));
    r.add(Bashrc::new(parameters.clone()));
    r.add(Info::new(
        "config sync finished. `Super+Shift+r` to reload desktop",
    ));

    r
}

pub fn qemu_list(parameters: Parameters) -> TaskRunner {
    // docs
    // qemu:    https://wiki.archlinux.org/title/QEMU
    // network: https://wiki.archlinux.org/title/Systemd-networkd#Bridge_interface
    //          https://www.freedesktop.org/software/systemd/man/systemd.network.html
    let mut r = TaskRunner::new();

    r.add(RequireUser::new("qemu_install", "root"));
    r.add(Command::new(
        "install_qemu_packages",
        "pacman -Sy --noconfirm \
            qemu-base \
            virt-manager \
            dmidecode",
        false,
        false,
    ));
    r.add(Command::new(
        "add_user_to_libvirt_group",
        &format!("usermod -a -G libvirt {}", parameters.username),
        false,
        false,
    ));

    r.add(TextFile::new(
        "/etc/systemd/network/qemu0.netdev",
        "[NetDev]
Name=qemu0
Kind=bridge",
    ));

    r.add(TextFile::new(
        "/etc/systemd/network/qemu0.network",
        "[Match]
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
DNS=1.1.1.1",
    ));

    let net_dev = parameters.net_dev;
    r.add(TextFile::new(
        &format!("/etc/systemd/network/qemu0-{net_dev}-uplink.network"),
        &format!(
            "[Match]
Name={net_dev}
[Network]
Bridge=br0"
        ),
    ));

    r.add(TextFile::new("/etc/qemu/bridge.conf", "allow qemu0"));

    r.add(Command::new(
        "enable_libvirtd_service",
        "systemctl enable libvirtd",
        false,
        false,
    ));
    r.add(Command::new(
        "start_networkd_and_libvirtd_services",
        "systemctl restart systemd-networkd libvirtd",
        false,
        false,
    ));
    r.add(Command::new(
        "print_services_status",
        "systemctl status systemd-networkd libvirtd | grep -E '(.service|Active:) '",
        true,
        true,
    ));
    r.add(Info::new("done, to open gui run `virt-manager`"));

    r
}
