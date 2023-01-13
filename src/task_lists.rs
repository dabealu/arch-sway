use crate::parameters::Parameters;
use crate::tasks::*;

pub fn installation_list(parameters: Parameters) -> TaskRunner {
    // TODO: fix Parameters lifetime, use reference instead clone
    let mut r = TaskRunner::new();

    let username = &parameters.username;

    //------------------//
    // Stage 1: chroot  //
    //------------------//
    r.add(Box::new(RequireUser::new("chroot", "root")));
    r.add(Box::new(WifiConnect::new(parameters.clone())));
    r.add(Box::new(Command::new(
        "install_git",
        "pacman -Sy --noconfirm git",
        false,
        false,
    )));
    r.add(Box::new(GitRepo::new(
        "arch_sway_repo",
        "https://github.com/dabealu/arch-sway.git",
        "arch-sway-repo",
    )));
    r.add(Box::new(Partitions::new(parameters.clone())));
    r.add(Box::new(FS::new(parameters.clone())));
    r.add(Box::new(Command::new(
        "pacstrap_packages",
        "pacstrap /mnt base base-devel linux linux-firmware \
            grub efibootmgr dosfstools os-prober mtools \
            systemd-resolvconf wpa_supplicant netplan \
            openssh dnsutils curl git unzip vim sudo man tmux \
            sysstat bash-completion go",
        false,
        false,
    )));
    r.add(Box::new(Command::new(
        "save_fstab",
        "genfstab -U /mnt >> /mnt/etc/fstab",
        false,
        true,
    )));
    // tasks within arch-chroot
    r.add(Box::new(Command::new(
        "set_timezone",
        &format!(
            "arch-chroot /mnt ln -sf /usr/share/zoneinfo/{} /etc/localtime && \
            arch-chroot /mnt hwclock --systohc",
            &parameters.timezone
        ),
        false,
        true,
    )));
    r.add(Box::new(Locales::new()));
    r.add(Box::new(Hostname::new(parameters.clone())));
    r.add(Box::new(User::new(parameters.clone())));
    r.add(Box::new(Grub::new(parameters.clone())));
    r.add(Box::new(Info::new(&format!(
        "next steps: reboot and run ./{BIN_FILE} as a root"
    ))));
    r.add(Box::new(StageCompleted::new(
        "chroot_stage_completed",
        "/mnt/root",
    )));

    //------------------//
    // Stage 2: install //
    //------------------//
    r.add(Box::new(RequireUser::new("install", "root")));
    r.add(Box::new(Command::new(
        "enable_ntp",
        "timedatectl set-ntp true",
        false,
        false,
    )));
    r.add(Box::new(Network::new(parameters.clone())));
    r.add(Box::new(Resolved::new()));
    r.add(Box::new(Netplan::new(parameters.clone())));
    // TODO: create pacman struct - check installed packages and install only diff
    r.add(Box::new(Command::new(
        "install_sway_packages",
        "pacman -Sy --noconfirm \
            sway swaylock swayidle waybar light xorg-xwayland \
            bemenu-wayland libnotify dunst wl-clipboard alacritty",
        false,
        false,
    )));
    r.add(Box::new(Info::new("base desktop installed")));
    r.add(Box::new(Command::new(
        "install_fonts_themes_utilities",
        "pacman -Sy --noconfirm \
            grim slurp ddcutil lxappearance \
            lshw pciutils usbutils mc \
            noto-fonts noto-fonts-emoji noto-fonts-extra \
            ttf-roboto opendesktop-fonts \
            materia-gtk-theme papirus-icon-theme adwaita-qt5",
        false,
        false,
    )));
    r.add(Box::new(Variables::new()));
    r.add(Box::new(SwayConfigs::new(parameters.clone())));
    r.add(Box::new(Command::new(
        "install_pipewire",
        "pacman -Sy --noconfirm \
            pipewire pipewire-pulse wireplumber \
            gst-plugin-pipewire xdg-desktop-portal-wlr",
        false,
        false,
    )));
    r.add(Box::new(Swap::new()));
    r.add(Box::new(Hibernation::new()));
    r.add(Box::new(TextFile::new(
        "/etc/sysctl.d/01-swappiness.conf",
        "vm.swappiness = 1",
    )));
    r.add(Box::new(CpuGovernor::new()));
    r.add(Box::new(Bluetooth::new()));
    r.add(Box::new(Docker::new(parameters.clone())));
    r.add(Box::new(Command::new(
        "install_rust_toolchain",
        &format!(
            "pacman -Sy --noconfirm rustup && \
            sudo -u {username} -- rustup default stable"
        ),
        false,
        true,
    )));
    r.add(Box::new(Command::new(
        "install_yay_aur",
        &format!(
            "sudo -u {username} -- bash -c 'mkdir -p ~/projects && cd ~/projects && \
            git clone https://aur.archlinux.org/yay-git.git && \
            cd yay-git && \
            makepkg --noconfirm -si'"
        ),
        false,
        true,
    )));
    r.add(Box::new(Command::new(
        "install_aur_packages",
        &format!(
            "sudo -u {username} -- bash -c 'yes | yay --noconfirm -Sy wdisplays libinput-gestures'"
        ),
        false,
        true,
    )));
    r.add(Box::new(Command::new(
        "add_user_to_input_group",
        &format!("usermod -aG input {username}"),
        false,
        false,
    )));
    r.add(Box::new(Command::new(
        "start_pipewire",
        &format!(
            "systemctl --user -M {username}@.host enable pipewire pipewire-pulse && \
            systemctl --user -M {username}@.host start pipewire pipewire-pulse"
        ),
        false,
        true,
    )));
    r.add(Box::new(Bashrc::new(parameters)));
    r.add(Box::new(Command::new(
        "install_flatpak_add_flathub_repo",
        &format!(
            "pacman -Sy --noconfirm flatpak && \
            flatpak remote-add --if-not-exists --system flathub https://flathub.org/repo/flathub.flatpakrepo"
        ),
        false,
        true,
    )));
    r.add(Box::new(FlatpakPackages::new()));
    r.add(Box::new(Info::new("installation finished")));

    r
}
