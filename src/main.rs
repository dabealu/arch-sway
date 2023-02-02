mod base_methods;
mod parameters;
mod paths;
mod task_lists;
mod tasks;

use std::env;
use task_lists::*;

const HELP_MESSAGE: &str = "available flags:
* l | list                - print tasks list without execution
* i | install             - run tasks to install the system
* t | start-from          - start installation from specific task
* c | clear-progress      - remove file with saved progress
* s | sync                - use git repo to sync configs 
* q | qemu                - install and configure qemu/kvm
* u | update-bin          - compile new bin from local repo
* b | build-iso           - create iso with arch-sway bin included
* f | format-dev dev iso  - format device to include storage and boot partitions";

fn main() {
    let mut args = env::args();
    let cmd = args.next().unwrap_or("arch-sway".to_string());
    match args.next() {
        Some(flag) => match flag.as_ref() {
            "list" | "l" => installation_list(parameters::Parameters::dummy()).list(),
            "install" | "i" => installation_list(parameters::Parameters::build()).run(),
            "sync" | "s" => sync_list(parameters::Parameters::build()).run(),
            "qemu" | "q" => qemu_list(parameters::Parameters::build()).run(),
            "start-from" | "t" => {
                if let Some(task_id) = args.next() {
                    installation_list(parameters::Parameters::build()).run_from(&task_id);
                } else {
                    println!("usage: {cmd} continue <task_id>");
                }
            }
            "clear-progress" | "c" => {
                if let Err(e) = tasks::clear_progress() {
                    println!("failed to clear current progress: {e}");
                }
            }
            "update-bin" | "u" => {
                if let Err(e) = tasks::update_bin() {
                    println!("failed to build new binary: {e}");
                }
            }
            "build-iso" | "b" => {
                if let Err(e) = tasks::create_iso() {
                    println!("failed to create iso: {e}");
                }
            }
            "format-dev" | "f" => {
                let dev_path = &args.next().expect("error: missing path to storage device");
                let iso_path = &args.next().expect("error: missing path to iso file");
                if let Err(e) = tasks::format_device(dev_path, iso_path) {
                    println!("failed to format storage device {dev_path}: {e}");
                }
            }
            _ => {
                println!("unknown flag '{flag}', {HELP_MESSAGE}")
            }
        },
        None => println!("specify flag, {HELP_MESSAGE}"),
    }
}
