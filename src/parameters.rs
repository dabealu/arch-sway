use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::{env, fmt, fs, io, path::Path, process, str};

use crate::base_methods::*;

pub const PARAMETERS_FILE: &str = "arch-sway-parameters.yaml";
const NETWORK_INTERFACES_REGEX: &str = r"^(wlan|wlp|eth|enp).*";

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Parameters {
    pub efi: bool,
    pub block_device: String,
    pub part_num: u8,            // 2 for EFI, 1 for BIOS
    pub part_num_prefix: String, // "" for sdaX, and "p" nvme0n1pX
    pub timezone: String,        // path inside /usr/share/zoneinfo/, e.g. "America/Toronto"
    pub hostname: String,
    pub username: String,
    pub user_id: String,
    pub user_gid: String,
    pub net_dev: String,
    pub net_dev_iso: String, // wlan0, eth0 in archiso renamed to wlp, enp after
    pub wifi_enabled: bool,
    pub wifi_ssid: String,
    pub wifi_password: String,
}

impl fmt::Display for Parameters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Ok(yaml) = serde_yaml::to_string(&self) {
            write!(f, "{}", yaml)
        } else {
            write!(f, "{:?}", &self)
        }
    }
}

impl Parameters {
    pub fn build() -> Parameters {
        // try to load parameters from file
        if let Ok(yaml_str) = fs::read_to_string(PARAMETERS_FILE) {
            let yaml_res: Result<Parameters, _> = serde_yaml::from_str(&yaml_str);
            if let Ok(params) = yaml_res {
                println!("got parameters from file '{PARAMETERS_FILE}'");
                return params;
            }
        }

        // otherwise, request user input
        // and save it to file for later use
        let params = Self::request_user_parameters();

        match serde_yaml::to_string(&params) {
            Ok(yaml) => fs::write(PARAMETERS_FILE, yaml)
                .expect(&format!("failed to save {PARAMETERS_FILE}")),
            Err(e) => panic!("failed to save installation parameters to file: {e}"),
        }

        params
    }

    pub fn dummy() -> Parameters {
        let s = "".to_string();
        Parameters {
            efi: true,
            block_device: s.clone(),
            part_num: 0,
            part_num_prefix: s.clone(),
            timezone: s.clone(),
            hostname: s.clone(),
            username: s.clone(),
            user_id: s.clone(),
            user_gid: s.clone(),
            net_dev: s.clone(),
            net_dev_iso: s.clone(),
            wifi_enabled: true,
            wifi_ssid: "MyWiFi".to_string(),
            wifi_password: s,
        }
    }

    fn request_user_parameters() -> Parameters {
        // TODO: deconstruct this fn into small fns
        println!("enter parameters:");

        // EFI
        let mut efi = false;
        if Path::new("/sys/firmware/efi").exists() {
            efi = true;
        }

        // block dev
        let block_devices = list_block_dev();
        if block_devices.is_empty() {
            panic!("fatal: no block devices found")
        }
        let mut block_dev = ask_user_input(&format!("block device {:?}:", block_devices));
        if block_dev.len() == 0 {
            block_dev = block_devices[0].clone();
        }

        // partition prefix - "" for sdaX, and "p" for nvme0n1pX
        let mut block_dev_prefix = "".to_string();
        if block_dev.starts_with("nvme") {
            block_dev_prefix = "p".to_string();
        }

        // timezone
        let default_tz = "America/Toronto";
        let mut tz = ask_user_input(&format!("timezone [{default_tz}]:"));
        if tz.len() == 0 {
            tz = default_tz.to_string();
        }

        // hostname
        let default_hostname = "dhost";
        let mut hostname = ask_user_input(&format!("hostname [{default_hostname}]:"));
        if hostname.len() == 0 {
            hostname = default_hostname.to_string();
        }

        // username
        let default_user = "user";
        let mut user = ask_user_input(&format!("username [{default_user}]:"));
        if user.len() == 0 {
            user = default_user.to_string();
        }

        // network interface
        let net_devices = list_net_dev().unwrap();
        if net_devices.is_empty() {
            panic!(
                "fatal: cannot find (regex: {}) any network interface!",
                NETWORK_INTERFACES_REGEX
            );
        }
        let net_dev_default = net_devices[0].clone();

        let mut net_dev_iso = ask_user_input(&format!(
            "network interface (available:{net_devices:?}) [{net_dev_default}]:"
        ));
        if net_dev_iso.len() == 0 {
            net_dev_iso = net_dev_default;
        }

        // wifi
        let mut default_wifi = false;
        if net_dev_iso.starts_with("wlan") || net_dev_iso.starts_with("wlp") {
            default_wifi = true;
        }
        let wifi_str = ask_user_input(&format!("configure wifi [{default_wifi}]"));
        let wifi = match wifi_str.as_ref() {
            "" => default_wifi,
            "true" => true,
            "false" => false,
            _ => panic!("configure wifi is a boolean parameter"), // TODO: move to separate fn and retry ask input
        };

        let mut wifi_ssid = "".to_string();
        let mut wifi_passwd = "".to_string();
        if wifi {
            wifi_ssid = env_or_input("WIFI_SSID", "wifi ssid:");
            wifi_passwd = env_or_input("WIFI_PASSWD", "wifi password:");
        }

        // archiso uses traditional interface naming like eth0, wla0, etc
        // but interfaces named by udev during later stages (e.g. enp, wlp),
        // so it's required to get "real" interface name during archiso.
        // example:
        // udevadm test-builtin net_id /sys/class/net/wlan0 | grep ID_NET_NAME_PATH
        // ID_NET_NAME_PATH=wlp1s0  <<<<<-------------^^^^^
        let udev_output = run_cmd(
            &format!("udevadm test-builtin net_id /sys/class/net/{net_dev_iso}"),
            true,
        )
        .unwrap();

        // TODO: consider moving "grepping" into separate fn
        let mut net_dev = "".to_string();
        for line in udev_output.lines() {
            if line.starts_with("ID_NET_NAME_PATH") {
                net_dev = line.split("=").collect::<Vec<&str>>()[1].to_string();
                println!("{net_dev_iso} will be named {net_dev} after archiso");
            }
        }

        let res = Parameters {
            efi: efi,
            block_device: block_dev,
            part_num: if efi { 2 } else { 1 },
            part_num_prefix: block_dev_prefix,
            timezone: tz,
            hostname: hostname,
            username: user,
            user_id: "1000".to_string(),
            user_gid: "1000".to_string(),
            net_dev: net_dev,
            net_dev_iso: net_dev_iso,
            wifi_enabled: wifi,
            wifi_ssid: wifi_ssid,
            wifi_password: wifi_passwd,
        };

        println!("\n---\nparameters:\n{res}");

        // ask for confirmation before install
        loop {
            let proceed = ask_user_input("proceed with the installation? [yn]");
            match proceed.to_lowercase().as_str() {
                "y" => break,
                "n" => {
                    println!("exiting...");
                    process::exit(0);
                }
                _ => {
                    println!("unknown input '{proceed}', please enter y or n");
                }
            }
        }

        res
    }
}

pub fn read_user_input() -> String {
    let mut input = String::new();
    let stdin = io::stdin();
    match stdin.read_line(&mut input) {
        Ok(_) => return input.trim().to_owned(),
        Err(e) => panic!("failed to read user's input: {e}"),
    }
}

pub fn ask_user_input(msg: &str) -> String {
    println!("{msg} ");
    return read_user_input();
}

fn env_or_input(var: &str, msg: &str) -> String {
    println!("{msg} ");
    let var_res = env::var(var);
    if var_res.is_ok() {
        println!("using value from '{var}' variable");
        var_res.unwrap()
    } else {
        read_user_input()
    }
}

fn list_net_dev() -> Result<Vec<String>, io::Error> {
    let mut res: Vec<String> = vec![];
    let re = Regex::new(NETWORK_INTERFACES_REGEX).unwrap();

    for entry_result in fs::read_dir("/sys/class/net/")?.into_iter() {
        if let Ok(entry) = entry_result?.file_name().into_string() {
            if re.is_match(&entry) {
                res.push(entry);
            }
        }
    }

    Ok(res)
}

fn list_block_dev() -> Vec<String> {
    match Command::new("lsblk")
        .args(["--output=NAME", "--noheadings", "--nodeps"])
        .output()
    {
        Ok(output) => match str::from_utf8(&output.stdout) {
            Ok(str_utf8) => {
                return {
                    let mut res: Vec<String> = vec![];
                    for s in str_utf8.lines() {
                        res.push(s.to_string())
                    }
                    res
                }
            }
            Err(e) => panic!("failed to get list of block devices: {e}"),
        },
        Err(e) => panic!("failed to get list of block devices: {e}"),
    }
}

// TODO: tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_block_device() {
        assert_eq!(list_block_dev(), vec!["nvme0n1".to_string()])
    }
}
