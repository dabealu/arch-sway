mod base_methods;
mod parameters;
mod task_lists;
mod tasks;

use task_lists::*;

fn main() {
    let p = parameters::Parameters::build();
    let r = installation_list(p);
    r.run();
}
