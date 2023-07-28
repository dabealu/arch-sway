use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::{fmt, fs, io, path::Path, str};

use crate::base_methods::*;
use crate::paths::*;

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
        let conf_dir = conf_dir("", "");
        if !Path::new(&conf_dir).exists() {
            create_dir(&conf_dir).unwrap();
        }

        let parameters_file = parameters_file("", "");

        // try to load parameters from file
        if let Ok(yaml_str) = fs::read_to_string(&parameters_file) {
            let yaml_res: Result<Parameters, _> = serde_yaml::from_str(&yaml_str);
            if let Ok(params) = yaml_res {
                println!("got parameters from '{parameters_file}'");
                return params;
            }
        }

        // otherwise, request user input
        // and save it to file for later use
        let params = Self::request_user_parameters();

        match serde_yaml::to_string(&params) {
            Ok(yaml) => fs::write(&parameters_file, yaml)
                .expect(&format!("failed to save {parameters_file}")),
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
        let efi = if Path::new("/sys/firmware/efi").exists() {
            true
        } else {
            false
        };

        // block dev
        let block_devices = list_block_dev();
        if block_devices.is_empty() {
            panic!("fatal: no block devices found")
        }
        let mut block_dev = ask_user_input(&format!("block device {:?}:", block_devices));
        if block_dev.is_empty() {
            block_dev = block_devices[0].clone();
        }

        // partition prefix - "" for sdaX, and "p" for nvme0n1pX
        let block_dev_prefix = if block_dev.starts_with("nvme") {
            "p".to_string()
        } else {
            "".to_string()
        };

        // timezone
        let default_tz = "America/Toronto".to_string();
        let mut tz = ask_user_input(&format!("timezone [{default_tz}]:"));
        if tz.is_empty() {
            tz = default_tz;
        }

        // hostname
        let default_hostname = "dhost".to_string();
        let mut hostname = ask_user_input(&format!("hostname [{default_hostname}]:"));
        if hostname.is_empty() {
            hostname = default_hostname;
        }

        // username
        let default_user = "user".to_string();
        let mut user = ask_user_input(&format!("username [{default_user}]:"));
        if user.is_empty() {
            user = default_user;
        }

        // network interface
        let net_devices = list_net_dev().unwrap();
        if net_devices.is_empty() {
            panic!("fatal: cannot find (regex: {NETWORK_INTERFACES_REGEX}) any network interface!");
        }
        let net_dev_default = net_devices[0].clone();

        let mut net_dev_iso = ask_user_input(&format!(
            "network interface (available:{net_devices:?}) [{net_dev_default}]:"
        ));
        if net_dev_iso.is_empty() {
            net_dev_iso = net_dev_default;
        }

        // wifi
        let default_wifi = if net_dev_iso.starts_with("wlan") || net_dev_iso.starts_with("wlp") {
            true
        } else {
            false
        };

        let wifi = match ask_user_input(&format!("configure wifi [{default_wifi}]")).as_ref() {
            "" => default_wifi,
            "true" => true,
            "false" => false,
            _ => panic!("configure wifi is a boolean parameter"), // TODO: move to separate fn and retry ask input
        };

        let (wifi_ssid, wifi_passwd) = if wifi {
            (
                env_or_input("WIFI_SSID", "wifi ssid:"),
                env_or_input("WIFI_PASSWD", "wifi password:"),
            )
        } else {
            ("".to_string(), "".to_string())
        };

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

        let p = Parameters {
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

        println!("\n▒▒ parameters:\n{p}");

        ask_confirmation("▒▒ proceed with the installation?");

        p
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
