use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::{env, fs, io, str, path::Path};

pub const PARAMETERS_FILE: &str = "arch-sway-parameters.yaml";

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
    pub network_interface: String,
    pub wifi_enabled: bool,
    pub wifi_ssid: String,
    pub wifi_password: String,
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

    fn request_user_parameters() -> Parameters {
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
        let mut net_dev = ask_user_input(&format!("network interface {:?}:", net_devices));
        if net_dev.len() == 0 {
            net_dev = net_devices[0].clone();
        }

        // wifi
        let mut default_wifi = false;
        if net_dev.starts_with("wlan") || net_dev.starts_with("wlp") {
            default_wifi = true;
        }
        let wifi_str = ask_user_input(&format!("use wifi [{default_wifi}]"));
        let wifi = match &wifi_str[..] {
            "" => default_wifi,
            "true" => true,
            "false" => false,
            _ => panic!("use wifi is a boolean parameter"), // TODO: move to separate fn and retry ask input
        };

        let mut wifi_ssid = "".to_string();
        let mut wifi_passwd = "".to_string();
        if wifi {
            wifi_ssid = env_or_input("WIFI_SSID", "wifi ssid:");
            wifi_passwd = env_or_input("WIFI_PASSWD", "wifi password:");
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
            network_interface: net_dev,
            wifi_enabled: wifi,
            wifi_ssid: wifi_ssid,
            wifi_password: wifi_passwd,
        };

        println!("{res:?}");
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
    let re = Regex::new(r"^(wlan|wlp|eth|enp).*").unwrap();

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

// -------------------------
// TODO: tests
// -------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_block_device() {
        assert_eq!(list_block_dev(), vec!["nvme0n1".to_string()])
    }
}
