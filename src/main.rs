mod base_methods;
mod parameters;
mod task_lists;
mod tasks;

use std::env;
use task_lists::*;

const HELP_MESSAGE: &str = "available flags:
* list             - print list of tasks without execution
* install          - run tasks to install system
* sync             - sync configs on installed system from git
* start-from       - start installation from specific task
* clear-progress   - remove file with saved progress";

fn main() {
    let mut args = env::args();
    let cmd = args.next().unwrap_or("arch-sway".to_string());
    match args.next() {
        Some(flag) => match flag.as_ref() {
            "list" => installation_list(parameters::Parameters::dummy()).list(),
            "install" => installation_list(parameters::Parameters::build()).run(),
            "sync" => {
                // TODO: re-sync config files from git on installed system
            }
            "start-from" => {
                if let Some(task_id) = args.next() {
                    installation_list(parameters::Parameters::build()).run_from(&task_id);
                } else {
                    println!("usage: {cmd} continue <task_id>");
                }
            }
            "clear-progress" => {
                if let Err(e) = tasks::clear_progress() {
                    println!("failed to clear current progress: {e}");
                }
            }
            _ => {
                println!("unknown flag '{flag}', {HELP_MESSAGE}")
            }
        },
        None => println!("specify flag, {HELP_MESSAGE}"),
    }
}
