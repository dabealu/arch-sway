use std::env;

use crate::base_methods::join_paths;

// paths:
//    chroot      /mnt
//    bin         /usr/local/bin/arch-sway
//    repo        ~/src/arch-sway
//    progress    ~/.arch-sway/progress
//    parameters  ~/.arch-sway/parameters.yaml
const BIN_FILE: &str = "/usr/local/bin/arch-sway";
const SRC_DIR: &str = "src";
const REPO_DIR: &str = "src/arch-sway";
const CONF_DIR: &str = ".arch-sway";
const PROGRESS_FILE: &str = "/progress";
const PARAMETERS_FILE: &str = "/parameters.yaml";

// assumed in functions args:
// - no chroot if `chroot` is empty
// - current user (get from env var) if `user` is empty

fn prefix_chroot(chroot: &str, path: &str) -> String {
    if !chroot.is_empty() {
        return join_paths(chroot, path);
    }
    return path.to_string();
}

fn prefix_home(user: &str, path: &str) -> String {
    let user = if user.is_empty() {
        // check if running under sudo, otherwise use normal user var
        match env::var("SUDO_USER") {
            Ok(val) => val,
            Err(_) => match env::var("USER") {
                Ok(val) => val,
                Err(e) => {
                    panic!("fatal: unable to get value for $USER env var: {e}");
                }
            },
        }
    } else {
        user.to_string()
    };

    let prefix = if user == "root" {
        "/root/".to_string()
    } else {
        format!("/home/{user}/")
    };

    return join_paths(&prefix, &path);
}

pub fn bin_file(chroot: &str) -> String {
    prefix_chroot(chroot, BIN_FILE)
}

pub fn src_dir(chroot: &str, user: &str) -> String {
    prefix_chroot(chroot, &prefix_home(user, SRC_DIR))
}

pub fn repo_dir(chroot: &str, user: &str) -> String {
    prefix_chroot(chroot, &prefix_home(user, REPO_DIR))
}

pub fn conf_dir(chroot: &str, user: &str) -> String {
    prefix_chroot(chroot, &prefix_home(user, CONF_DIR))
}

pub fn progress_file(chroot: &str, user: &str) -> String {
    conf_dir(chroot, user) + PROGRESS_FILE
}

pub fn parameters_file(chroot: &str, user: &str) -> String {
    conf_dir(chroot, user) + PARAMETERS_FILE
}

// TODO:
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_chroot() {
        assert_eq!(prefix_chroot("", "tmp"), "tmp");
        assert_eq!(prefix_chroot("/mnt", "tmp"), "/mnt/tmp");
        assert_eq!(prefix_chroot("/mnt", "/tmp"), "/mnt/tmp");
    }

    #[test]
    fn test_prefix_home() {
        assert_eq!(prefix_home("user", "apps"), "/home/user/apps");
        assert_eq!(prefix_home("root", "apps"), "/root/apps");
        assert_eq!(prefix_home("root", "/apps"), "/root/apps");
    }

    #[test]
    fn test_path_to_bin_file() {
        assert_eq!(bin_file(""), "/usr/local/bin/arch-sway");
        assert_eq!(bin_file("/mnt"), "/mnt/usr/local/bin/arch-sway");
    }

    #[test]
    fn test_path_to_src_dir() {
        assert_eq!(src_dir("", "user"), "/home/user/src");
        assert_eq!(src_dir("", "root"), "/root/src");
        assert_eq!(src_dir("/mnt", "user"), "/mnt/home/user/src");
    }

    #[test]
    fn test_path_to_repo_dir() {
        assert_eq!(repo_dir("", "user"), "/home/user/src/arch-sway");
        assert_eq!(repo_dir("", "root"), "/root/src/arch-sway");
        assert_eq!(repo_dir("/mnt", "user"), "/mnt/home/user/src/arch-sway");
    }

    #[test]
    fn test_path_to_conf_dir() {
        assert_eq!(conf_dir("", "user"), "/home/user/.arch-sway");
        assert_eq!(conf_dir("", "root"), "/root/.arch-sway");
        assert_eq!(conf_dir("/mnt", "user"), "/mnt/home/user/.arch-sway");
    }

    #[test]
    fn test_path_to_progress_file() {
        assert_eq!(progress_file("", "user"), "/home/user/.arch-sway/progress");
        assert_eq!(progress_file("", "root"), "/root/.arch-sway/progress");
        assert_eq!(
            progress_file("/mnt", "user"),
            "/mnt/home/user/.arch-sway/progress"
        );
    }
}
