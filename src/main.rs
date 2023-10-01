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
* s | sync                - (sudo) sync configs and desktop settings
* q | qemu                - install and configure qemu/kvm
* u | update-bin          - compile new bin from local repo
* b | build-iso           - create iso with arch-sway bin included
* f | format-dev dev iso  - format device creating storage and boot partitions
* e | steam vga           - (sudo) install steam, vga options: intel, nvidia, amd
* v | version             - print version and exit";

fn main() {
    let mut args = env::args();
    let cmd = args.next().unwrap_or("arch-sway".to_string());
    match args.next() {
        Some(flag) => match flag.as_ref() {
            "l" | "list" => installation_list(parameters::Parameters::dummy()).list(),
            "i" | "install" => installation_list(parameters::Parameters::build()).run(),
            "s" | "sync" => {
                if let Err(e) = tasks::clear_progress() {
                    panic!("failed to clear current progress: {e}");
                }
                sync_list(parameters::Parameters::build()).run();
            }
            "q" | "qemu" => {
                if let Err(e) = tasks::clear_progress() {
                    panic!("failed to clear current progress: {e}");
                }
                qemu_list(parameters::Parameters::build()).run()
            }
            // TODO: to run task T it's required to actually specify task previous to T which is confusing
            "t" | "start-from" => {
                if let Some(task_id) = args.next() {
                    installation_list(parameters::Parameters::build()).run_from(&task_id);
                } else {
                    println!("usage: {cmd} start-from <task_id>");
                }
            }
            "c" | "clear-progress" => {
                if let Err(e) = tasks::clear_progress() {
                    println!("failed to clear current progress: {e}");
                }
            }
            "u" | "update-bin" => {
                if let Err(e) = tasks::update_bin() {
                    println!("failed to build new binary: {e}");
                }
            }
            "b" | "build-iso" => {
                if let Err(e) = tasks::create_iso(parameters::Parameters::build()) {
                    println!("failed to create iso: {e}");
                }
            }
            "f" | "format-dev" => {
                let dev_path = &args.next().expect("error: missing path to storage device");
                let iso_path = &args.next().expect("error: missing path to iso file");
                if let Err(e) = tasks::format_device(dev_path, iso_path) {
                    println!("failed to format storage device {dev_path}: {e}");
                }
            }
            "v" | "version" => {
                println!("v{}", env!("CARGO_PKG_VERSION"));
            }
            "e" | "steam" => {
                let vga_type = &args.next().expect("error: missing vga type");
                if let Err(e) = tasks::install_steam(vga_type) {
                    println!("failed to install steam: {e}");
                }
            }
            _ => {
                println!("unknown flag '{flag}', {HELP_MESSAGE}")
            }
        },
        None => println!("missing flag, {HELP_MESSAGE}"),
    }
}
