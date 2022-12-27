use crate::parameters::Parameters;
use crate::tasks::*;

pub fn installation_list(parameters: Parameters) -> TaskRunner {
    // TODO: fix Parameters lifetime, use reference instead clone
    let mut r = TaskRunner::new();

    //------------------//
    // Stage 1: chroot  //
    //------------------//
    r.add(Box::new(RequireUser::new("chroot", "root")));
    r.add(Box::new(Command::new(
        "install_git",
        "pacman -Sy --noconfirm archlinux-keyring git",
        false,
        false,
    )));
    r.add(Box::new(GitRepo::new(
        "clone_arch_sway_repo",
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
        "next steps: \
        \n\treboot, run as a root \
        \n\t./{}\n",
        BIN_FILE
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
            bemenu-wayland libnotify dunst wl-clipboard",
        false,
        false,
    )));
    r.add(Box::new(Command::new(
        "install_fonts_themes_utilities",
        "pacman -Sy --noconfirm \
            grim slurp ddcutil lxappearance \
            lshw pciutils usbutils \
            ttf-liberation ttf-roboto ttf-dejavu noto-fonts \
            noto-fonts-emoji noto-fonts-extra opendesktop-fonts \
            materia-gtk-theme papirus-icon-theme adwaita-qt5",
        false,
        false,
    )));
    r.add(Box::new(Variables::new()));
    r.add(Box::new(SwayConfigs::new(parameters.clone())));
    r.add(Box::new(Command::new(
        "install_desktop_apps",
        "pacman -Sy --noconfirm \
            alacritty code telegram-desktop \
            thunar evince xournalpp ristretto \
            transmission-gtk audacious vlc",
        false,
        false,
    )));
    r.add(Box::new(Command::new(
        "install_pipewire",
        "pacman -Sy --noconfirm \
            pipewire pipewire-pulse wireplumber \
            gst-plugin-pipewire xdg-desktop-portal-wlr \
            pavucontrol",
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
    r.add(Box::new(Info::new(&format!(
        "next steps: \
        \n\treboot and run as a regular user: \
        \n\t./{}\n",
        BIN_FILE
    ))));
    r.add(Box::new(StageCompleted::new(
        "install_stage_completed",
        &format!("/home/{}/", &parameters.username),
    )));

    //---------------//
    // Stage 3: user //
    //---------------//
    // TODO: lot of bash it this stage
    r.add(Box::new(Command::new(
        "chown_moved_files",
        &format!(
            "sudo chown -R {}:{} ~/*",
            parameters.username, parameters.username
        ),
        false,
        true,
    )));
    r.add(Box::new(RequireUser::new("nonroot", &parameters.username)));
    r.add(Box::new(Command::new(
        "install_rust_toolchain",
        "sudo pacman -Sy --noconfirm rustup && \
        rustup default stable",
        false,
        true,
    )));
    r.add(Box::new(Command::new(
        "install_yay_aur",
        "mkdir -p ~/projects && cd ~/projects && \
        git clone https://aur.archlinux.org/yay-git.git && \
        cd yay-git && \
        makepkg --noconfirm -si",
        false,
        true,
    )));
    r.add(Box::new(Command::new(
        "install_aur_packages",
        "yes | yay --noconfirm -Sy wdisplays google-chrome libinput-gestures",
        false,
        true,
    )));
    r.add(Box::new(Command::new(
        "add_user_to_input_group",
        &format!("sudo usermod -aG input {}", parameters.username),
        false,
        false,
    )));
    r.add(Box::new(Command::new(
        "enable_and_start_pipewire",
        "systemctl --user enable pipewire.service pipewire-pulse.service && \
        systemctl --user start pipewire.service pipewire-pulse.service",
        false,
        true,
    )));
    r.add(Box::new(Bashrc::new(parameters)));
    r.add(Box::new(Info::new("installation finished successfully!")));
    r.add(Box::new(StageCompleted::new("user_stage_completed", "")));

    r
}
