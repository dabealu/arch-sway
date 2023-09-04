use chrono::Utc;
use std::collections::HashMap;
use std::fs::Permissions;
use std::os::unix::prelude::PermissionsExt;
use std::{error::Error, io::ErrorKind, path::Path};
use std::{fmt, fs, process, thread, time};

use crate::parameters::Parameters;
use crate::{base_methods::*, paths};

pub fn save_progress(dest: &str, task: &str) -> Result<(), TaskError> {
    let dest = if dest.is_empty() {
        paths::progress_file("", "")
    } else {
        dest.to_string()
    };

    let dest_path = Path::new::<String>(&dest);
    if !dest_path.exists() {
        let path_err = Err(TaskError::new("failed to get progress file directory path"));

        match dest_path.parent() {
            Some(dir_path) => match dir_path.to_str() {
                Some(path_str) => {
                    create_dir(path_str)?;
                }
                None => {
                    return path_err;
                }
            },
            None => {
                return path_err;
            }
        }
    }

    if let Err(e) = fs::write(dest, task) {
        return Err(TaskError::new(&e.to_string()));
    }
    Ok(())
}

pub fn load_progress() -> Result<String, TaskError> {
    match fs::read_to_string(paths::progress_file("", "")) {
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

pub fn clear_progress() -> Result<(), TaskError> {
    let progress_file_path = paths::progress_file("", "");
    if is_file_exist(&progress_file_path) {
        if let Err(e) = fs::remove_file(&progress_file_path) {
            return Err(TaskError::new(&format!(
                "failed to remove {progress_file_path}: {e}"
            )));
        }
    }
    Ok(())
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

    pub fn list(&self) {
        for t in &self.tasks {
            let task_name = t.name();
            if !task_name.is_empty() {
                println!("\x1b[94m\x1b[1m▒▒ {task_name}\x1b[0m");
            }
        }
    }

    pub fn run_from(&self, task_id: &str) {
        if let Err(e) = save_progress("", task_id) {
            println!("failed to overwrite current task: {e}");
            process::exit(1);
        }
        self.run();
    }

    pub fn run(&self) {
        let mut task_saved = match load_progress() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("\x1b[93m\x1b[1m▒▒ warning: failed to load progress: \x1b[0m{e}");
                "".to_string()
            }
        };

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

            // actual task execution
            let run_result = t.run();

            match t.signal() {
                TaskSignal::StageCompleted(progress_file_moved_to) => {
                    // run() moves files to new dir if specified
                    if let Err(e) = run_result {
                        println!("\x1b[91m\x1b[1m▒▒ error:\x1b[0m {e}");
                        process::exit(1);
                    }
                    if let Err(e) = save_progress(&progress_file_moved_to, &task_name) {
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

                // checking required user task shouldn't be saved to progress
                TaskSignal::RequireUser => match run_result {
                    Ok(_) => {
                        println!("\x1b[92m\x1b[1m▒▒ ok\x1b[0m");
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
    RequireUser,
    StageCompleted(String),
}

pub struct Command {
    name: String,
    command: String,
    output: bool,
    shell: bool,
}

impl Command {
    pub fn new(name: &str, command: &str, output: bool, shell: bool) -> Box<dyn Task> {
        Box::new(Command {
            name: name.to_string(),
            command: command.to_string(),
            output: output,
            shell: shell,
        })
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
    pub fn new(msg: &str) -> Box<dyn Task> {
        Box::new(Info {
            message: msg.to_string(),
        })
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
    chroot: String,
    user: String,
}

impl StageCompleted {
    pub fn new(name: &str, chroot: &str, user: &str) -> Box<dyn Task> {
        Box::new(StageCompleted {
            name: name.to_string(),
            chroot: chroot.to_string(),
            user: user.to_string(),
        })
    }
}

impl Task for StageCompleted {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn signal(&self) -> TaskSignal {
        let progress_file_moved_to = paths::progress_file(&self.chroot, &self.user);
        TaskSignal::StageCompleted(progress_file_moved_to)
    }

    fn run(&self) -> Result<String, TaskError> {
        let ok = Ok("".to_string());

        if self.chroot.is_empty() && self.user.is_empty() {
            return ok;
        }

        let repo_dir = &paths::repo_dir("", "");
        let src_dir = &paths::src_dir("", "");
        let conf_dir = &paths::conf_dir("", "");
        let repo_dir_dest = &paths::repo_dir(&self.chroot, &self.user);
        let src_dir_dest = &paths::src_dir(&self.chroot, &self.user);
        let conf_dir_dest = &paths::conf_dir(&self.chroot, &self.user);

        create_dir(src_dir_dest)?;

        if !self.chroot.is_empty() {
            let bin_file = match std::env::current_exe() {
                Ok(exe_path) => exe_path.display().to_string(),
                Err(_) => paths::bin_file(""),
            };

            let bin_file_dest = paths::bin_file(&self.chroot);
            println!("moving binary {bin_file} -> {bin_file_dest}");
            // TODO: get rid of bash
            run_cmd(&format!("mv {bin_file} {bin_file_dest}"), false)?;
        }

        // moving repo_dir instead of src, otherwise it will result in `src/src/arch-sway`
        if !is_file_exist(repo_dir_dest) {
            println!("moving repo dir {repo_dir} -> {repo_dir_dest}");
            run_cmd(&format!("mv {repo_dir} {repo_dir_dest}"), false)?;
        }

        if !is_file_exist(conf_dir_dest) {
            println!("moving conf dir {conf_dir} -> {conf_dir_dest}");
            run_cmd(&format!("mv {conf_dir} {conf_dir_dest}"), false)?;
        }

        if !self.user.is_empty() {
            run_cmd(
                &format!(
                    "chown -R {}:{} {src_dir_dest} {conf_dir_dest}",
                    self.user, self.user
                ),
                false,
            )?;
            symlink(conf_dir_dest, conf_dir)?;
            symlink(src_dir_dest, src_dir)?;
        }

        ok
    }
}

pub struct Locales;

impl Locales {
    pub fn new() -> Box<dyn Task> {
        Box::new(Locales {})
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
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(Hostname {
            parameters: parameters,
        })
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
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(User {
            parameters: parameters,
        })
    }
}

impl Task for User {
    fn name(&self) -> String {
        "create_user".to_string()
    }

    fn signal(&self) -> TaskSignal {
        TaskSignal::RequireUser
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
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(Grub {
            parameters: parameters,
        })
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
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(FS {
            parameters: parameters,
        })
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
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(Partitions {
            parameters: parameters,
        })
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
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(Network {
            parameters: parameters,
        })
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
                self.parameters.net_dev
            ),
            &format!(
                "[Match] \
                \nName={} \
                \n[Network] \
                \nDHCP=yes\n",
                self.parameters.net_dev
            ),
        )?;

        run_cmd("systemctl enable systemd-networkd", false)?;
        run_cmd("systemctl start systemd-networkd", false)
    }
}

pub struct Resolved;

impl Resolved {
    pub fn new() -> Box<dyn Task> {
        Box::new(Resolved)
    }
}

impl Task for Resolved {
    fn name(&self) -> String {
        "configure_systemd_resolved".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        line_in_file("/etc/resolv.conf", "nameserver 127.0.0.53")?;

        create_dir("/etc/systemd/resolved.conf.d")?;

        copy_file(
            &format!("{}/assets/files/dns_servers.conf", paths::repo_dir("", "")),
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
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(Netplan {
            parameters: parameters,
        })
    }
}

// TODO: get rid of netplan in favor of networkd
impl Task for Netplan {
    fn name(&self) -> String {
        "netplan_configuration".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        run_cmd("pacman -Sy --noconfirm dbus-python python-rich", false)?;

        create_dir("/etc/netplan")?;

        if self.parameters.wifi_enabled {
            // TODO: use proper templating
            match fs::read_to_string(format!(
                "{}/assets/files/netplan-wifi-config.yaml",
                paths::repo_dir("", "")
            )) {
                Ok(template) => {
                    let content = template
                        .replace("_NETWORK_INTERFACE_", &self.parameters.net_dev)
                        .replace("_WIFI_SSID_", &self.parameters.wifi_ssid)
                        .replace("_WIFI_PASSWORD_", &self.parameters.wifi_password);
                    text_file("/etc/netplan/wifi-config.yaml", &content)?;
                }
                Err(e) => return Err(TaskError::new(&e.to_string())),
            }
        } else {
            match fs::read_to_string(format!(
                "{}/assets/files/netplan-eth-config.yaml",
                paths::repo_dir("", "")
            )) {
                Ok(template) => {
                    let content = template.replace("_NETWORK_INTERFACE_", &self.parameters.net_dev);
                    text_file("/etc/netplan/eth-config.yaml", &content)?;
                }
                Err(e) => return Err(TaskError::new(&e.to_string())),
            }
        }

        run_cmd("netplan apply", false)?;
        // wait a bit until network is fully up
        thread::sleep(time::Duration::from_secs(3));
        run_cmd("netplan get all", true)
    }
}

pub struct Variables;

impl Variables {
    pub fn new() -> Box<dyn Task> {
        Box::new(Variables)
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
            "XDG_CURRENT_DESKTOP=sway",
            "XDG_SESSION_TYPE=wayland",
            "WLR_DRM_NO_MODIFIERS=1",
            "# WLR_RENDERER=vulkan",
            "QT_QPA_PLATFORM=wayland",
            "QT_WAYLAND_DISABLE_WINDOWDECORATION=1",
            "QT_STYLE_OVERRIDE=Adwaita-dark",
            "GTK_THEME=Materia:dark",
            "CLUTTER_BACKEND=wayland",
            "SDL_VIDEODRIVER=wayland",
            "ELM_DISPLAY=wl",
            "ELM_ACCEL=opengl",
            "ECORE_EVAS_ENGINE=wayland_egl",
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
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(SwayConfigs {
            parameters: parameters,
        })
    }
}

impl Task for SwayConfigs {
    fn name(&self) -> String {
        "create_sway_config_files".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let username = &self.parameters.username;

        create_dir(&format!("/home/{username}/.config/sway/conf.d"))?;

        match fs::read_dir(format!("{}/assets/conf", paths::repo_dir("", ""))) {
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
                                &format!("{}/assets/conf/{conf_filename}", paths::repo_dir("", "")),
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
            &format!(
                "chown -R {}:{} /home/{username}",
                &self.parameters.user_id, &self.parameters.user_gid
            ),
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
    pub fn new(stage: &str, user: &str) -> Box<dyn Task> {
        Box::new(RequireUser {
            stage: stage.to_string(),
            user: user.to_string(),
        })
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
                        "required user: {}, current user: {}",
                        self.user, current_user
                    )));
                } else {
                    return Ok("".to_string());
                }
            }
        }
    }
}

pub struct GitRepo;

impl GitRepo {
    pub fn new() -> Box<dyn Task> {
        Box::new(GitRepo {})
    }
}

impl Task for GitRepo {
    fn name(&self) -> String {
        "arch_sway_git_repo".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let dest = paths::repo_dir("", "");
        let repo = "https://github.com/dabealu/arch-sway.git";

        if Path::new::<String>(&dest).exists() {
            return Ok(format!(
                "using local repo {dest}, note that it may have discrepancies with the remote"
            ));
        }

        println!("cloning {repo} to {dest}");
        run_cmd(&format!("git clone {repo} {dest}"), false)
    }
}

pub struct Swap;

impl Swap {
    pub fn new() -> Box<dyn Task> {
        Box::new(Swap)
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
    pub fn new() -> Box<dyn Task> {
        Box::new(Hibernation)
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
    pub fn new(path: &str, content: &str) -> Box<dyn Task> {
        Box::new(TextFile {
            path: path.to_string(),
            content: content.to_string(),
        })
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
    pub fn new() -> Box<dyn Task> {
        Box::new(CpuGovernor)
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
    pub fn new() -> Box<dyn Task> {
        Box::new(Bluetooth)
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
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(Docker {
            parameters: parameters,
        })
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
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(Bashrc {
            parameters: parameters,
        })
    }
}

impl Task for Bashrc {
    fn name(&self) -> String {
        "bashrc_and_user_bin_dir".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let bin_dir = &format!("/home/{}/bin", self.parameters.username);
        create_dir(&bin_dir)?;

        let bashrc_path = &format!("/home/{}/.bashrc", self.parameters.username);
        copy_file(
            &format!("{}/assets/files/bashrc", paths::repo_dir("", "")),
            bashrc_path,
        )?;

        run_cmd(
            &format!(
                "chown -R {}:{} {bin_dir} {bashrc_path}",
                self.parameters.user_id, self.parameters.user_gid
            ),
            false,
        )?;

        Ok("".to_string())
    }
}

pub struct InstallUtils {
    parameters: Parameters,
}

impl InstallUtils {
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(InstallUtils {
            parameters: parameters,
        })
    }
}

impl Task for InstallUtils {
    fn name(&self) -> String {
        "install_utilities".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let home_bin_dir = &format!("/home/{}/bin", self.parameters.username);

        run_cmd(
            &format!(
                "go build -o {home_bin_dir}/brightness-control {}/brightness_control.go",
                join_paths(&paths::repo_dir("", ""), "tools")
            ),
            false,
        )?;

        run_cmd(
            &format!(
                "chown -R {}:{} {home_bin_dir}",
                self.parameters.user_id, self.parameters.user_gid
            ),
            false,
        )
    }
}

pub struct ConfigureIDE {
    parameters: Parameters,
}

impl ConfigureIDE {
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(ConfigureIDE {
            parameters: parameters,
        })
    }
}

impl Task for ConfigureIDE {
    fn name(&self) -> String {
        "configure_editor".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let user = &self.parameters.username;
        let extensions = vec![
            "golang.go",
            "rust-lang.rust-analyzer",
            "GitHub.github-vscode-theme",
            "PKief.material-icon-theme",
            "ecmel.vscode-html-css",
            // "vscodevim.vim",
        ];

        for ext in extensions {
            run_cmd(
                &format!("sudo -u {user} -- code --install-extension {ext}"),
                false,
            )?;
        }

        for conf_file in vec!["settings.json", "keybindings.json"] {
            copy_file(
                &format!("{}/assets/files/{conf_file}", paths::repo_dir("", "")),
                &format!("/home/{user}/.config/Code - OSS/User/{conf_file}"),
            )?;
        }

        Ok("".to_string())
    }
}

pub struct WifiConnect {
    parameters: Parameters,
}

impl WifiConnect {
    pub fn new(parameters: Parameters) -> Box<dyn Task> {
        Box::new(WifiConnect {
            parameters: parameters,
        })
    }
}

impl Task for WifiConnect {
    fn name(&self) -> String {
        format!("connect_to_wifi_ssid:{}", &self.parameters.wifi_ssid)
    }

    fn run(&self) -> Result<String, TaskError> {
        // skip if wifi disabled or network is already configured
        if !self.parameters.wifi_enabled {
            return Ok("".to_string());
        }

        // route command produces no output (len=0) if default route not present
        if let Ok(route) = run_cmd("ip route show default", true) {
            if route.len() > 0 {
                return Ok("".to_string());
            }
        }

        let wlan = &self.parameters.net_dev_iso;
        run_shell(
            &format!(
                "ip link set {wlan} up && \
                wpa_supplicant -B -i {wlan} -c <(wpa_passphrase '{}' '{}') && \
                dhcpcd",
                self.parameters.wifi_ssid, self.parameters.wifi_password
            ),
            false,
        )?;

        // pause until network is up
        thread::sleep(time::Duration::from_secs(3));
        Ok("".to_string())
    }
}

// flatpak as an alternative to native packages.
// requires flatpak package and remote to be added:
//      pacman -Sy --noconfirm flatpak && \
//      flatpak remote-add --if-not-exists --system flathub https://flathub.org/repo/flathub.flatpakrepo"
// themes in flatpak apps, doesn't work very well at the moment:
// https://docs.flatpak.org/en/latest/desktop-integration.html#installing-themes
pub struct FlatpakPackages;

impl FlatpakPackages {
    #[allow(dead_code)]
    pub fn new() -> Box<dyn Task> {
        Box::new(FlatpakPackages)
    }
}

impl Task for FlatpakPackages {
    fn name(&self) -> String {
        "install_flatpak_packages".to_string()
    }

    fn run(&self) -> Result<String, TaskError> {
        let apps = HashMap::from([
            ("com.google.Chrome", "chrome"),
            ("com.visualstudio.code", "code"),
            ("org.atheme.audacious", "audacious"),
            ("org.gnome.Evince", "evince"),
            ("org.telegram.desktop", "telegram"),
            ("com.github.xournalpp.xournalpp", "xournalpp"),
            ("org.xfce.ristretto", "ristretto"),
            ("com.github.PintaProject.Pinta", "pinta"),
            ("com.transmissionbt.Transmission", "transmission"),
            ("org.videolan.VLC", "vlc"),
            ("org.pulseaudio.pavucontrol", "pavucontrol"),
        ]);

        for (id, name) in apps {
            println!("\t{name} - {id}");

            // install flatpak package
            run_cmd(
                &format!("flatpak install --system --noninteractive --assumeyes flathub {id}"),
                false,
            )?;

            // create symlink with short name
            symlink(
                &format!("/var/lib/flatpak/exports/bin/{id}"),
                &format!("/usr/local/bin/{name}"),
            )?;

            // adjust package permissions
            run_cmd(
                &format!(
                    "flatpak override {id} \
                    --device=all \
                    --filesystem=host \
                    --share=network \
                    --share=ipc \
                    --socket=wayland \
                    --socket=pulseaudio \
                    --socket=session-bus"
                ),
                false,
            )?;
        }

        Ok("".to_string())
    }
}

pub fn create_iso(parameters: Parameters) -> Result<(), TaskError> {
    let releng_dir = "/usr/share/archiso/configs/releng";
    let repo_dir = paths::repo_dir("", "");
    let bin_path = join_paths(&repo_dir, "target/release/arch-sway");
    let archiso_dir = join_paths(&repo_dir, "archiso");
    let iso_builds_dir = join_paths(&repo_dir, "iso-builds");
    let releng_copy_dir = join_paths(&archiso_dir, "releng");
    let releng_bin_path = join_paths(&releng_copy_dir, "airootfs/usr/local/bin/arch-sway");
    let profiledef_path = join_paths(&releng_copy_dir, "profiledef.sh");

    println!("building a binary");
    build_bin()?;

    // remove archiso from previous build, it may still exist if previous build failed
    run_cmd(&format!("sudo rm -rf {archiso_dir}"), false)?;
    create_dir(&archiso_dir)?;

    // TODO: implement copy_dir
    run_shell(&format!("cp -r {releng_dir} {archiso_dir}"), false)?;
    copy_file(&bin_path, &releng_bin_path)?;

    replace_line(
        &profiledef_path,
        r##"file_permissions=\("##,
        &format!("file_permissions=(\n  [\"/usr/local/bin/arch-sway\"]=\"0:0:755\""),
    )?;

    println!("building iso, it may take a while...");
    // mkarchiso must be run as root
    let script = &format!(
        "cd {archiso_dir} && sudo mkarchiso -v -w . -o {iso_builds_dir} {releng_copy_dir}"
    );
    println!("running: {script}");
    run_shell(&script, false)?;

    run_cmd(
        &format!(
            "sudo chown -R {}:{} {iso_builds_dir}",
            parameters.user_id, parameters.user_gid
        ),
        false,
    )?;
    run_cmd(&format!("sudo rm -rf {archiso_dir}"), false)?;

    println!("{}", run_cmd("lsblk", true)?);
    println!("done, to create installation media run:\nsudo cp {iso_builds_dir}/archlinux-YYYY.MM.DD-x86_64.iso /dev/sdX");

    Ok(())
}

pub fn update_bin() -> Result<(), TaskError> {
    build_bin()?;

    let bin_src = join_paths(&paths::repo_dir("", ""), "target/release/arch-sway");
    let bin_dest = paths::bin_file("");
    run_cmd(&format!("sudo cp -f {bin_src} {bin_dest}"), false)?;

    println!("done. bin path: {bin_dest}");
    Ok(())
}

fn build_bin() -> Result<(), TaskError> {
    run_shell(
        &format!(
            "cd {} && \
            cargo fmt && \
            cargo test && \
            export ARCHSWAY_RELEASE_VERSION={} && \
            cargo build -r",
            paths::repo_dir("", ""),
            Utc::now().format("%Y.%m.%d-%H.%M.%S")
        ),
        false,
    )?;
    Ok(())
}

pub fn format_device(dev_path: &str, iso_path: &str) -> Result<(), TaskError> {
    // doc: https://wiki.archlinux.org/title/USB_flash_installation_medium#In_GNU/Linux_2
    // current implementation is for efi-gpt only.
    // boot partition must have proper filesystem label.
    // parse label from path to iso file, format ARCH_YYYYMM, e.g. ARCH_202210
    // path and iso name example: /home/user/Downloads/archlinux-2022.10.01-x86_64.iso
    let parse_err =
        "unable to parse date from iso file name, expected format: archlinux-2022.10.01-x86_64.iso";

    let date_vec = Path::new(iso_path)
        .file_name()
        .expect("unable to get file name from iso path")
        .to_str()
        .expect("unable to parse filepath into str")
        .split("-")
        .collect::<Vec<&str>>() // -> [archlinux, 2022.10.01, x86_64.iso]
        .get(1)
        .expect(parse_err)
        .split(".")
        .collect::<Vec<&str>>(); // -> [2022, 10, 01]

    let iso_label = format!(
        "ARCH_{}{}",
        date_vec.get(0).expect(parse_err),
        date_vec.get(1).expect(parse_err),
    );

    ask_confirmation(&format!(
        "warning: this will wipe data from {dev_path}, continue?"
    ));

    // create partition and fs for iso
    println!("creating partitions");
    let parted = format!("sudo parted -s {dev_path}");
    run_cmd(&format!("{parted} mklabel gpt"), false)?;
    run_cmd(
        &format!("{parted} mkpart Arch_ISO fat32 1MiB 1024MiB"),
        false,
    )?;

    run_cmd(&format!("sudo mkfs.fat -F 32 {dev_path}1"), false)?;
    run_cmd(&format!("sudo fatlabel {dev_path}1 {iso_label}"), false)?;

    // mount fs located in the storage device and extract the contents of the iso image to it
    println!("copying iso to {dev_path}1");
    let mnt_dir = join_paths(&paths::repo_dir("", ""), "iso-device-mnt");

    create_dir(&mnt_dir).unwrap();

    run_cmd(&format!("sudo mount {dev_path}1 {mnt_dir}"), false)?;
    run_cmd(&format!("sudo bsdtar -x -f {iso_path} -C {mnt_dir}"), false)?;

    // unmount fs, install the syslinux and mtools packages and make the partition bootable
    run_cmd(&format!("sudo umount {mnt_dir}"), false)?;
    run_cmd(
        &format!("sudo syslinux --directory syslinux --install {dev_path}1"),
        false,
    )?;
    run_cmd(
        &format!(
            "sudo dd bs=440 count=1 conv=notrunc \
            if=/usr/lib/syslinux/bios/gptmbr.bin \
            of={dev_path}"
        ),
        false,
    )?;

    // allocate rest of the space to storage partition
    run_cmd(
        &format!("{parted} mkpart FlashDrive ext4 1024MiB 100%"),
        false,
    )?;
    run_cmd(&format!("sudo mkfs.ext4 {dev_path}2"), false)?;

    // rm temporary mount dir
    if let Err(e) = fs::remove_dir_all(&mnt_dir) {
        println!("failed to remove directory {mnt_dir}: {e}");
        process::exit(1);
    }
    println!("done");

    Ok(())
}
