use std::fs::Permissions;
use std::os::unix::prelude::PermissionsExt;
use std::{error::Error, fmt, fs, io::ErrorKind, path::Path, process};

use crate::parameters::Parameters;
use crate::{base_methods::*, parameters};

pub const BIN_FILE: &str = "arch-sway";
pub const REPO_PATH: &str = "arch-sway-repo";
pub const PROGRESS_FILE: &str = "arch-sway-progress";

pub fn save_progress(dir: &str, task: &str) -> Result<(), TaskError> {
    let mut dest = PROGRESS_FILE.to_string();
    if !dir.is_empty() {
        if let Some(s) = Path::new(dir).join(PROGRESS_FILE).to_str() {
            dest = s.to_string();
        }
    }

    if let Err(e) = fs::write(dest, task) {
        return Err(TaskError::new(&e.to_string()));
    }
    Ok(())
}

pub fn load_progress() -> Result<String, TaskError> {
    match fs::read_to_string(PROGRESS_FILE) {
        Ok(s) => Ok(s.trim().to_string()),
        Err(e) => {
            match e.kind() {
                // ignore if file doesn't exist
                ErrorKind::NotFound => Ok("".to_string()),
                _ => Err(TaskError::new(&e.to_string())),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskError {
    message: String,
}

impl TaskError {
    pub fn new(message: &str) -> TaskError {
        TaskError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl Error for TaskError {}

pub struct TaskRunner {
    tasks: Vec<Box<dyn Task>>,
}

impl TaskRunner {
    pub fn new() -> TaskRunner {
        TaskRunner { tasks: vec![] }
    }

    pub fn add(&mut self, task: Box<dyn Task>) {
        self.tasks.push(task);
    }

    pub fn run(&self) {
        let mut task_saved = "".to_string();
        match load_progress() {
            Ok(s) => task_saved = s,
            Err(e) => println!("\x1b[93m\x1b[1m▒▒ warning: failed to load status: \x1b[0m{e}"),
        }

        for t in &self.tasks {
            let task_name = t.name();

            if !task_name.is_empty() {
                println!("\x1b[94m\x1b[1m▒▒ {task_name}\x1b[0m");
            }

            // skip all task before previous execution task reached
            if !task_name.is_empty() && !task_saved.is_empty() && task_name != task_saved {
                println!("\x1b[2m\x1b[1m▒▒ skipped\x1b[0m");
                continue;
            }

            // previous execution task reached, reset status, continue run
            if !task_saved.is_empty() && task_name == task_saved {
                task_saved = "".to_string();
                println!("\x1b[2m\x1b[1m▒▒ skipped\x1b[0m");
                continue;
            }

            let run_result = t.run();

            match t.signal() {
                TaskSignal::StageCompleted(move_to) => {
                    // run() moves files to new dir if specified
                    if let Err(e) = run_result {
                        println!("\x1b[91m\x1b[1m▒▒ error:\x1b[0m {e}");
                        process::exit(1);
                    }
                    if let Err(e) = save_progress(&move_to, &task_name) {
                        println!("\x1b[93m\x1b[1m▒▒ warning: failed to save status: \x1b[0m{e}")
                    }
                    process::exit(0);
                }

                TaskSignal::Info => match run_result {
                    Ok(output) => {
                        println!("{}", output.trim());
                        continue;
                    }
                    Err(e) => {
                        println!("\x1b[91m\x1b[1m▒▒ error:\x1b[0m {e}");
                        process::exit(1);
                    }
                },

                TaskSignal::Default => match run_result {
                    Ok(output) => {
                        println!("\x1b[92m\x1b[1m▒▒ ok\x1b[0m");
                        if !output.is_empty() {
                            println!("{}", output.trim());
                        }
                        if let Err(e) = save_progress("", &task_name) {
                            println!("\x1b[93m\x1b[1m▒▒ warning: failed to save status: \x1b[0m{e}")
                        }
                    }
                    Err(e) => {
                        println!("\x1b[91m\x1b[1m▒▒ error:\x1b[0m {e}");
                        process::exit(1);
                    }
                },
            }
        }
    }
}

pub trait Task {
    fn name(&self) -> String {
        "".to_string()
    }

    // TODO: implement for other types to print some debug info during run
    // e.g.: command prints actual shell command, or line in file prints line, etc
    // add description to be printed in TaskRunner
    fn description(&self) -> String {
        "".to_string()
    }

    fn signal(&self) -> TaskSignal {
        TaskSignal::Default
    }

    fn run(&self) -> Result<String, TaskError>;
}

pub enum TaskSignal {
    Default,
    Info,
    StageCompleted(String),
}

pub struct Command {
    name: String,
    command: String,
    output: bool,
    shell: bool,
}

impl Command {
    pub fn new(name: &str, command: &str, output: bool, shell: bool) -> Command {
        Command {
            name: name.to_string(),
            command: command.to_string(),
            output: output,
            shell: shell,
        }
    }
}

impl Task for Command {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn run(&self) -> Result<String, TaskError> {
        if self.shell {
            return run_shell(&self.command, self.output);
        } else {
            return run_cmd(&self.command, self.output);
        }
    }
}

pub struct Info {
    message: String,
}

impl Info {
    pub fn new(msg: &str) -> Info {
        Info {
            message: msg.to_string(),
        }
    }
}

impl Task for Info {
    fn name(&self) -> String {
        "".to_string()
    }

    fn signal(&self) -> TaskSignal {
        TaskSignal::Info
    }

    fn run(&self) -> Result<String, TaskError> {
        Ok(self.message.to_string())
    }
}

pub struct StageCompleted {
    name: String,
    move_to: String,
}

impl StageCompleted {
    pub fn new(name: &str, move_to: &str) -> StageCompleted {
        StageCompleted {
            name: name.to_string(),
            move_to: move_to.to_string(),
        }
    }
}

impl Task for StageCompleted {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn signal(&self) -> TaskSignal {
        TaskSignal::StageCompleted(self.move_to.to_string())
    }

    fn run(&self) -> Result<String, TaskError> {
        if !self.move_to.is_empty() {
            // TODO: get rid of bash for move
            run_cmd(
                &format!(
                    "mv {} {} {} {} {}",
                    BIN_FILE,
                    PROGRESS_FILE,
                    REPO_PATH,
                    parameters::PARAMETERS_FILE,
                    self.move_to
                ),
                false,
            )?;
        }
        Ok("".to_string())
    }
}

pub struct Locales;

impl Locales {
    pub fn new() -> Locales {
        Locales {}
    }
}

impl Task for Locales {
    fn name(&self) -> String {
        "configure_locales".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        replace_line(
            "/mnt/etc/locale.gen",
            r"# *ru_RU.UTF-8 UTF-8",
            "ru_RU.UTF-8 UTF-8",
        )?;

        replace_line(
            "/mnt/etc/locale.gen",
            r"# *en_US.UTF-8 UTF-8",
            "en_US.UTF-8 UTF-8",
        )?;

        run_cmd("arch-chroot /mnt locale-gen", false)?;

        line_in_file("/mnt/etc/locale.conf", "LANG=en_US.UTF-8")?;
        line_in_file("/mnt/etc/vconsole.conf", "KEYMAP=ru")
    }
}

pub struct Hostname {
    parameters: Parameters,
}

impl Hostname {
    pub fn new(parameters: Parameters) -> Hostname {
        Hostname {
            parameters: parameters,
        }
    }
}

impl Task for Hostname {
    fn name(&self) -> String {
        "set_hostname".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let hostname = &self.parameters.hostname;
        text_file("/mnt/etc/hostname", hostname)?;
        // TODO: use proper templating
        text_file(
            "/mnt/etc/hosts",
            &format!(
                "# Static table lookup for hostnames. \
                # See hosts(5) for details. \
                127.0.0.1	localhost \
                ::1		localhost \
                127.0.1.1	{}.localdomain	{}\n",
                hostname, hostname
            ),
        )
    }
}

pub struct User {
    parameters: Parameters,
}

impl User {
    pub fn new(parameters: Parameters) -> User {
        User {
            parameters: parameters,
        }
    }
}

impl Task for User {
    fn name(&self) -> String {
        "create_user".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let username = self.parameters.username.to_string();
        let userid = self.parameters.user_id.to_string();
        let usergid = self.parameters.user_gid.to_string();

        // TODO: use stdlib for user management?
        run_shell(
            &format!(
                "arch-chroot /mnt groupadd -g {usergid} {username} && \
                arch-chroot /mnt useradd -m -u {userid} -g {usergid} {username} && \
                arch-chroot /mnt usermod -aG wheel,audio,video,storage {username}"
            ),
            false,
        )?;

        // add user to sudoers
        replace_line(
            "/mnt/etc/sudoers",
            "^# *%wheel.*NOPASSWD.*$",
            "%wheel ALL=(ALL:ALL) NOPASSWD: ALL",
        )?;

        // set default password for user and root
        run_shell(
            &format!(r##"arch-chroot /mnt bash -c "echo -e '1\n1' | passwd {username}""##),
            false,
        )?;
        run_shell(
            r##"arch-chroot /mnt bash -c "echo -e '1\n1' | passwd root""##,
            false,
        )?;

        Ok(format!(
            "warning: root and {username} passwords are set to '1'"
        ))
    }
}

pub struct Grub {
    parameters: Parameters,
}

impl Grub {
    pub fn new(parameters: Parameters) -> Grub {
        Grub {
            parameters: parameters,
        }
    }
}

impl Task for Grub {
    fn name(&self) -> String {
        "install_grub_bootloader".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        run_cmd("mkdir -p /mnt/boot/grub", false)?;

        if self.parameters.efi {
            // TODO: use separate functions instead of shell scripts
            run_shell(
                &format!(
                    "mkdir -p /mnt/boot/EFI && \
                    arch-chroot /mnt mount /dev/{}{}1 /boot/EFI && \
                    arch-chroot /mnt grub-install --target=x86_64-efi --bootloader-id=grub_uefi --recheck",
                    self.parameters.block_device, self.parameters.part_num_prefix
                ),
                false,
            )?;
        } else {
            run_cmd(
                &format!(
                    "arch-chroot /mnt grub-install --recheck --target=i386-pc /dev/{}",
                    self.parameters.block_device
                ),
                false,
            )?;
        }

        run_cmd(
            "arch-chroot /mnt grub-mkconfig -o /boot/grub/grub.cfg",
            false,
        )
    }
}

pub struct FS {
    parameters: Parameters,
}

impl FS {
    pub fn new(parameters: Parameters) -> FS {
        FS {
            parameters: parameters,
        }
    }
}

impl Task for FS {
    fn name(&self) -> String {
        "create_filesystems".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let dev = format!(
            "/dev/{}{}",
            self.parameters.block_device, self.parameters.part_num_prefix,
        );
        let rootfs_partition = format!("{dev}{}", self.parameters.part_num);

        if self.parameters.efi {
            // efi requires additional fat32 partition, e.g.:
            // /dev/nvme0n1p1 - EFI, /dev/nvme0n1p2 - root fs
            run_cmd(&format!("mkfs.fat -F 32 {dev}1"), false)?;
        }

        run_cmd(&format!("mkfs.ext4 {rootfs_partition}"), false)?;
        run_cmd(&format!("mount {rootfs_partition} /mnt"), false)?;

        run_cmd(
            &format!("parted -s /dev/{} print", self.parameters.block_device),
            true,
        )
    }
}

pub struct Partitions {
    parameters: Parameters,
}

impl Partitions {
    pub fn new(parameters: Parameters) -> Partitions {
        Partitions {
            parameters: parameters,
        }
    }
}

impl Task for Partitions {
    fn name(&self) -> String {
        "create_partitions".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let parted = format!("parted -s /dev/{}", self.parameters.block_device);

        if self.parameters.efi {
            run_cmd(&format!("{parted} mklabel gpt"), false)?;
            run_cmd(
                &format!("{parted} mkpart efi-system fat32 1MiB 512MiB"),
                false,
            )?;
            run_cmd(&format!("{parted} mkpart rootfs ext4 512MiB 100%"), false)?;
        } else {
            run_cmd(&format!("{parted} mklabel msdos"), false)?;
            run_cmd(&format!("{parted} mkpart primary ext4 1MiB 100%"), false)?;
        }

        run_cmd(&format!("{parted} set 1 boot on"), false)
    }
}

pub struct Network {
    parameters: Parameters,
}

impl Network {
    pub fn new(parameters: Parameters) -> Network {
        Network {
            parameters: parameters,
        }
    }
}

impl Task for Network {
    fn name(&self) -> String {
        "configure_network".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        text_file(
            &format!(
                "/etc/systemd/network/{}-dhcp.network",
                self.parameters.network_interface
            ),
            &format!(
                "[Match] \
                \nName={} \
                \n[Network] \
                \nDHCP=yes\n",
                self.parameters.network_interface
            ),
        )?;

        run_cmd("systemctl enable systemd-networkd", false)?;
        run_cmd("systemctl start systemd-networkd", false)
    }
}

pub struct Resolved;

impl Resolved {
    pub fn new() -> Resolved {
        Resolved
    }
}

impl Task for Resolved {
    fn name(&self) -> String {
        "configure_systemd_resolved".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        line_in_file("/etc/resolv.conf", "nameserver 127.0.0.53")?;

        if let Err(e) = std::fs::create_dir_all("/etc/systemd/resolved.conf.d") {
            return Err(TaskError::new(&format!("failed to create directory {e}")));
        }

        copy_file(
            &format!("{REPO_PATH}/assets/files/dns_servers.conf"),
            "/etc/systemd/resolved.conf.d/dns_servers.conf",
        )?;

        run_cmd("systemctl enable systemd-resolved", false)?;
        run_cmd("systemctl start systemd-resolved", false)
    }
}

pub struct Netplan {
    parameters: Parameters,
}

impl Netplan {
    pub fn new(parameters: Parameters) -> Netplan {
        Netplan {
            parameters: parameters,
        }
    }
}

impl Task for Netplan {
    fn name(&self) -> String {
        "netplan_configuration".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        if let Err(e) = std::fs::create_dir_all("/etc/netplan") {
            return Err(TaskError::new(&format!("failed to create directory {e}")));
        }

        if self.parameters.wifi_enabled {
            // TODO: use proper templating
            match fs::read_to_string(format!("{REPO_PATH}/assets/files/netplan-wifi-config.yaml")) {
                Ok(template) => {
                    let content = template
                        .replace("_NETWORK_INTERFACE_", &self.parameters.network_interface)
                        .replace("_WIFI_SSID_", &self.parameters.wifi_ssid)
                        .replace("_WIFI_PASSWORD_", &self.parameters.wifi_password);
                    text_file("/etc/netplan/wifi-config.yaml", &content)?;
                }
                Err(e) => return Err(TaskError::new(&e.to_string())),
            }
        } else {
            match fs::read_to_string(format!("{REPO_PATH}/assets/files/netplan-eth-config.yaml")) {
                Ok(template) => {
                    let content =
                        template.replace("_NETWORK_INTERFACE_", &self.parameters.network_interface);
                    text_file("/etc/netplan/eth-config.yaml", &content)?;
                }
                Err(e) => return Err(TaskError::new(&e.to_string())),
            }
        }

        run_cmd("netplan apply", false)?;
        run_cmd("netplan get all", true)
    }
}

pub struct Variables;

impl Variables {
    pub fn new() -> Variables {
        Variables
    }
}

impl Task for Variables {
    fn name(&self) -> String {
        "add_global_env_variables".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let vars = vec![
            "EDITOR=vim",
            "LIBSEAT_BACKEND=logind",
            "GTK_THEME=Materia:dark",
            "QT_STYLE_OVERRIDE=Adwaita-dark",
            "WLR_DRM_NO_MODIFIERS=1",
            "XDG_CURRENT_DESKTOP=sway",
            "XDG_SESSION_TYPE=wayland",
            "QT_QPA_PLATFORM=wayland",
            "QT_WAYLAND_DISABLE_WINDOWDECORATION=1",
        ];
        for var in vars {
            if let Err(e) = line_in_file("/etc/environment", var) {
                return Err(TaskError::new(&e.to_string()));
            }
        }
        Ok("".to_string())
    }
}

pub struct SwayConfigs {
    parameters: Parameters,
}

impl SwayConfigs {
    pub fn new(parameters: Parameters) -> SwayConfigs {
        SwayConfigs {
            parameters: parameters,
        }
    }
}

impl Task for SwayConfigs {
    fn name(&self) -> String {
        "create_sway_config_files".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let username = &self.parameters.username;

        if let Err(e) = std::fs::create_dir_all(format!("/home/{}/.config/sway/conf.d", username)) {
            return Err(TaskError::new(&format!("failed to create directory {e}")));
        }

        match fs::read_dir(format!("{REPO_PATH}/assets/conf")) {
            Ok(read_dir) => {
                for read_dir_res in read_dir.into_iter() {
                    match read_dir_res {
                        Ok(dir_entry) => {
                            let conf_filename = match dir_entry.file_name().to_str() {
                                Some(str) => str.to_string(),
                                None => {
                                    return Err(TaskError::new("failed to parse config filename"))
                                }
                            };

                            copy_file(
                                &format!("{REPO_PATH}/assets/conf/{conf_filename}"),
                                &format!("/home/{username}/.config/sway/{conf_filename}"),
                            )?;
                        }
                        Err(e) => return Err(TaskError::new(&e.to_string())),
                    }
                }
            }
            Err(e) => return Err(TaskError::new(&e.to_string())),
        }

        // TODO: use https://doc.rust-lang.org/stable/std/os/unix/fs/fn.chown.html - currently nightly
        run_cmd(
            &format!("chown -R {username}:{username} /home/{username}"),
            false,
        )?;
        run_cmd(
            &format!("chmod +x /home/{username}/.config/sway/waybar.sh"),
            false,
        )
    }
}

pub struct RequireUser {
    stage: String,
    user: String,
}

impl RequireUser {
    pub fn new(stage: &str, user: &str) -> RequireUser {
        RequireUser {
            stage: stage.to_string(),
            user: user.to_string(),
        }
    }
}

impl Task for RequireUser {
    fn name(&self) -> String {
        format!("check_required_user_{}_for_{}", self.user, self.stage)
    }

    fn run(&self) -> Result<String, TaskError> {
        match run_cmd("id -un", true) {
            Err(e) => Err(TaskError::new(&format!("cannot get current user: {e}"))),
            Ok(current_user) => {
                let current_user = current_user.trim();
                if current_user != self.user {
                    return Err(TaskError::new(&format!(
                        "current user: {}, required user: {}",
                        current_user, self.user
                    )));
                } else {
                    return Ok("".to_string());
                }
            }
        }
    }
}

pub struct GitRepo {
    name: String,
    dest: String,
    url: String,
}

impl GitRepo {
    pub fn new(name: &str, url: &str, dest: &str) -> GitRepo {
        GitRepo {
            name: name.to_string(),
            url: url.to_string(),
            dest: dest.to_string(),
        }
    }
}

impl Task for GitRepo {
    fn name(&self) -> String {
        format!("clone_git_repo_{}", self.name)
    }

    fn run(&self) -> Result<String, TaskError> {
        run_shell(&format!("git clone {} {}", self.url, self.dest), false)
    }
}

pub struct Swap;

impl Swap {
    pub fn new() -> Swap {
        Swap
    }
}

impl Task for Swap {
    fn name(&self) -> String {
        "create_swap_file".to_string()
    }

    // swap/hibernation doc: https://wiki.archlinux.org/title/Power_management/Suspend_and_hibernate#Hibernation
    // create swap file with the same size as RAM
    fn run(&self) -> Result<String, TaskError> {
        // grep MemTotal: /proc/meminfo
        // MemTotal:       32577276 kB
        match fs::read_to_string("/proc/meminfo") {
            Ok(lines) => {
                for line in lines.split("\n") {
                    if line.starts_with("MemTotal:") {
                        let mem_size_kb = line.split_whitespace().collect::<Vec<&str>>()[1];
                        run_cmd(&format!("fallocate -l {mem_size_kb}K /swapfile"), false)?;
                        break;
                    }
                }
            }
            Err(e) => return Err(TaskError::new(&e.to_string())),
        }

        if let Err(e) = fs::set_permissions("/swapfile", Permissions::from_mode(0o600)) {
            return Err(TaskError::new(&e.to_string()));
        }

        run_cmd("mkswap /swapfile", false)?;
        run_cmd("swapon /swapfile", false)?;
        line_in_file("/etc/fstab", "/swapfile none swap defaults 0 0")
    }
}

pub struct Hibernation;

impl Hibernation {
    pub fn new() -> Hibernation {
        Hibernation
    }
}

impl Task for Hibernation {
    fn name(&self) -> String {
        "enable_hibernation_and_suspend".to_string()
    }

    // TODO: this changes require reboot - add this info to Ok string?
    fn run(&self) -> Result<String, TaskError> {
        // initial ramdisk
        // order of hooks is important, see config comments for more details
        match fs::read_to_string("/etc/mkinitcpio.conf") {
            Ok(content) => {
                for line in content.lines() {
                    if line.starts_with("HOOKS=") && !line.ends_with("resume)") {
                        // grep '^HOOKS=' /etc/mkinitcpio.conf
                        // HOOKS=(base udev autodetect modconf block filesystems keyboard fsck)
                        let new_line = line.replace(")", " resume)");
                        if let Err(e) =
                            fs::write("/etc/mkinitcpio.conf", content.replace(line, &new_line))
                        {
                            return Err(TaskError::new(&e.to_string()));
                        }
                        break;
                    }
                }
            }
            Err(e) => return Err(TaskError::new(&e.to_string())),
        }
        run_cmd("mkinitcpio -p linux", false)?;

        // grub swap parameters
        let swap_file_device = run_cmd("sudo findmnt -no UUID -T /swapfile", true)?;

        let mut swap_file_offset = "";
        let output = run_cmd("sudo filefrag -v /swapfile", true)?;

        for line in output.lines() {
            let fields = line.split_whitespace().collect::<Vec<&str>>();
            if fields.starts_with(&["0:"]) {
                swap_file_offset = fields[3].trim_end_matches("..");
                break;
            }
        }

        let grub_params = &format!(
            r##"GRUB_CMDLINE_LINUX_DEFAULT="loglevel=3 quiet resume=UUID={} resume_offset={}""##,
            swap_file_device.trim(),
            swap_file_offset.trim()
        );

        // default: GRUB_CMDLINE_LINUX_DEFAULT="loglevel=3 quiet"
        replace_line(
            "/etc/default/grub",
            "GRUB_CMDLINE_LINUX_DEFAULT=.*",
            grub_params,
        )?;

        run_cmd("grub-mkconfig -o /boot/grub/grub.cfg", false)
    }
}

pub struct TextFile {
    path: String,
    content: String,
}

impl TextFile {
    pub fn new(path: &str, content: &str) -> TextFile {
        TextFile {
            path: path.to_string(),
            content: content.to_string(),
        }
    }
}

impl Task for TextFile {
    fn name(&self) -> String {
        format!("text_file:{}", self.path)
    }

    fn run(&self) -> Result<String, TaskError> {
        text_file(&self.path, &self.content)
    }
}

pub struct CpuGovernor;

impl CpuGovernor {
    pub fn new() -> CpuGovernor {
        CpuGovernor
    }
}

impl Task for CpuGovernor {
    fn name(&self) -> String {
        "set_performance_cpu_governor".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        // cpu doesn't support freq/governor parameters, ignore
        if !Path::new("/sys/devices/system/cpu/cpu0/cpufreq").exists() {
            return Ok("cpu doesn't support cpufreq control - missing /sys/devices/system/cpu/cpu0/cpufreq".to_string());
        }

        run_cmd("pacman -Sy --noconfirm cpupower", false)?;
        replace_line(
            "/etc/default/cpupower",
            "#governor='ondemand'",
            "governor='performance'",
        )?;
        run_cmd("systemctl enable cpupower", false)?;
        run_cmd("systemctl start cpupower", false)?;
        run_cmd(
            "cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor",
            true,
        )
    }
}

pub struct Bluetooth;

impl Bluetooth {
    pub fn new() -> Bluetooth {
        Bluetooth
    }
}

impl Task for Bluetooth {
    fn name(&self) -> String {
        "setup_bluetooth".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        run_cmd(
            "pacman -Sy --noconfirm bluez bluez-tools bluez-utils blueman",
            false,
        )?;
        // auto power-on adapter after boot
        replace_line(
            "/etc/bluetooth/main.conf",
            "# *AutoEnable *=.*",
            "AutoEnable = true",
        )?;
        run_cmd("systemctl enable bluetooth", false)?;
        run_cmd("systemctl start bluetooth", false)
    }
}

pub struct Docker {
    parameters: Parameters,
}

impl Docker {
    pub fn new(parameters: Parameters) -> Docker {
        Docker {
            parameters: parameters,
        }
    }
}

impl Task for Docker {
    fn name(&self) -> String {
        "setup_docker".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        run_cmd("pacman -Sy --noconfirm docker docker-compose", false)?;
        run_cmd(
            &format!("usermod -aG docker {}", self.parameters.username),
            false,
        )?;
        run_cmd("systemctl enable docker", false)?;
        run_cmd("systemctl start docker", false)
    }
}

pub struct Bashrc {
    parameters: Parameters,
}

impl Bashrc {
    pub fn new(parameters: Parameters) -> Bashrc {
        Bashrc {
            parameters: parameters,
        }
    }
}

impl Task for Bashrc {
    fn name(&self) -> String {
        "bashrc_and_user_bin_dir".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        if let Err(e) = fs::create_dir_all(format!("/home/{}/bin", self.parameters.username)) {
            return Err(TaskError::new(&e.to_string()));
        }

        copy_file(
            &format!("{REPO_PATH}/assets/files/bashrc"),
            &format!("/home/{}/.bashrc", self.parameters.username),
        )?;
        copy_file(
            &format!("{REPO_PATH}/assets/files/brightness.sh"),
            &format!("/home/{}/bin/brightness.sh", self.parameters.username),
        )?;

        Ok("".to_string())
    }
}
