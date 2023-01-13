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
    r.add(GitRepo::new(
        "arch_sway_repo",
        "https://github.com/dabealu/arch-sway.git",
        "arch-sway-repo",
    ));
    r.add(Partitions::new(parameters.clone()));
    r.add(FS::new(parameters.clone()));
    r.add(Command::new(
        "pacstrap_packages",
        "pacstrap /mnt base base-devel linux linux-firmware \
            grub efibootmgr dosfstools os-prober mtools \
            systemd-resolvconf wpa_supplicant netplan \
            openssh dnsutils curl git unzip vim sudo man tmux \
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
    r.add(Info::new(&format!(
        "next steps: reboot and run ./{BIN_FILE} as a root"
    )));
    r.add(StageCompleted::new("chroot_stage_completed", "/mnt/root"));

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
        "install_fonts_themes_utilities",
        "pacman -Sy --noconfirm \
            grim slurp ddcutil lxappearance \
            lshw pciutils usbutils mc \
            noto-fonts noto-fonts-emoji noto-fonts-extra \
            ttf-roboto opendesktop-fonts \
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
            "sudo -u {username} -- bash -c 'mkdir -p ~/projects && cd ~/projects && \
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
            "sudo -u {username} -- bash -c 'yes | yay --noconfirm -Sy wdisplays libinput-gestures'"
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
    r.add(Bashrc::new(parameters));
    r.add(Command::new(
        "install_flatpak_add_flathub_repo",
        &format!(
            "pacman -Sy --noconfirm flatpak && \
            flatpak remote-add --if-not-exists --system flathub https://flathub.org/repo/flathub.flatpakrepo"
        ),
        false,
        true,
    ));
    r.add(FlatpakPackages::new());
    r.add(Info::new("installation finished"));

    r
}
