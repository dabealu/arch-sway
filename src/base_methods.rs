use regex::Regex;
use std::{env, fs, io, io::Write, path::Path, process};

use crate::tasks::TaskError;

pub fn is_file_exist(path: &str) -> bool {
    match fs::metadata(path) {
        Ok(_) => true,
        Err(_) => false,
    }
}

// create file with specified content
pub fn text_file(path: &str, content: &str) -> Result<String, TaskError> {
    match fs::write(path, content) {
        Ok(_) => return Ok("".to_string()),
        Err(e) => return Err(TaskError::new(&e.to_string())),
    }
}

// append line to file
pub fn append_line(path: &str, line: &str) -> Result<String, TaskError> {
    match fs::OpenOptions::new().append(true).write(true).open(path) {
        Ok(mut file) => match writeln!(file, "{}", line) {
            Ok(_) => return Ok("".to_string()),
            Err(e) => return Err(TaskError::new(&e.to_string())),
        },
        Err(e) => return Err(TaskError::new(&e.to_string())),
    }
}

// append line if it's not found in file
pub fn line_in_file(path: &str, line: &str) -> Result<String, TaskError> {
    if !Path::new(path).exists() {
        return text_file(path, line);
    }

    if let Ok(txt) = fs::read_to_string(path) {
        for l in txt.lines() {
            if line == l {
                return Ok("".to_string());
            }
        }
    }

    append_line(path, line)
}

pub fn replace_line(path: &str, regex: &str, replace: &str) -> Result<String, TaskError> {
    let re = Regex::new(regex).unwrap();

    let mut result_lines: Vec<String> = vec![];
    if let Ok(txt) = fs::read_to_string(path) {
        for line in txt.lines() {
            if re.is_match(&line) {
                let new_line = re.replace(&line, replace);
                result_lines.push(new_line.to_string());
            } else {
                result_lines.push(line.to_string());
            }
        }
    }

    match fs::write(path, result_lines.join("\n")) {
        Ok(_) => Ok("".to_string()),
        Err(e) => Err(TaskError::new(&format!(
            "failed to replace line in `{path}`: {e}"
        ))),
    }
}

pub fn run_cmd(cmd: &str, output: bool) -> Result<String, TaskError> {
    let cmd_slice: Vec<&str> = cmd.split_whitespace().collect();

    if cmd_slice.is_empty() {
        return Err(TaskError::new("command cannot be empty"));
    }

    let args_slice: Vec<&str> = if cmd_slice.len() > 1 {
        cmd_slice[1..].to_vec()
    } else {
        vec![]
    };

    match process::Command::new(cmd_slice[0])
        .args(&args_slice)
        .output()
    {
        Ok(output_res) => {
            if output_res.status.success() {
                if output {
                    Ok(String::from_utf8(output_res.stdout).unwrap_or_default())
                } else {
                    Ok("".to_string())
                }
            } else {
                return Err(TaskError::new(&format!(
                    "failed to run \"{}\":\n{}\nstdout:\n{}\nstderr:\n{}\n",
                    cmd,
                    output_res.status,
                    String::from_utf8(output_res.stdout).unwrap_or_default(),
                    String::from_utf8(output_res.stderr).unwrap_or_default()
                )));
            }
        }
        Err(e) => Err(TaskError::new(&format!("failed to run `{cmd}`: {e}"))),
    }
}

// runs everything in a bash subshell, allowing to run scripts like `foo && bar | fizz`
pub fn run_shell(script: &str, output: bool) -> Result<String, TaskError> {
    let args_slice: Vec<&str> = vec!["-ec", script];

    match process::Command::new("bash").args(&args_slice).output() {
        Ok(output_res) => {
            if output_res.status.success() {
                if output {
                    Ok(String::from_utf8(output_res.stdout).unwrap_or_default())
                } else {
                    Ok("".to_string())
                }
            } else {
                return Err(TaskError::new(&format!(
                    "failed to run \"{}\":\n{}\nstdout:\n{}\nstderr:\n{}\n",
                    script,
                    output_res.status,
                    String::from_utf8(output_res.stdout).unwrap_or_default(),
                    String::from_utf8(output_res.stderr).unwrap_or_default()
                )));
            }
        }
        Err(e) => Err(TaskError::new(&format!("failed to run `{script}`: {e}"))),
    }
}

pub fn copy_file(from: &str, to: &str) -> Result<String, TaskError> {
    if let Err(e) = fs::copy(from, to) {
        return Err(TaskError::new(&format!(
            "failed to copy `{from}` to `{to}`: {e}"
        )));
    }
    Ok("".to_string())
}

pub fn symlink(origin: &str, link: &str) -> Result<String, TaskError> {
    if !Path::new::<str>(link).exists() {
        if let Err(e) = std::os::unix::fs::symlink(origin, link) {
            return Err(TaskError::new(&format!(
                "failed to create symlink from `{origin}` to `{link}`: {e}"
            )));
        }
    }
    Ok("".to_string())
}

pub fn create_dir(path: &str) -> Result<(), TaskError> {
    if let Err(e) = std::fs::create_dir_all(path) {
        return Err(TaskError::new(&format!(
            "failed to create directory {path}: {e}"
        )));
    }
    Ok(())
}

pub fn join_paths(a: &str, b: &str) -> String {
    a.trim_end_matches("/").to_string() + "/" + &b.trim_start_matches("/")
}

fn read_user_input() -> String {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => return input.trim().to_owned(),
        Err(e) => panic!("failed to read user's input from stdin: {e}"),
    }
}

pub fn ask_user_input(msg: &str) -> String {
    println!("{msg} ");
    return read_user_input();
}

pub fn env_or_input(var: &str, msg: &str) -> String {
    println!("{msg} ");
    let var_res = env::var(var);
    if var_res.is_ok() {
        println!("using value from '{var}' variable");
        var_res.unwrap()
    } else {
        read_user_input()
    }
}

pub fn ask_confirmation(msg: &str) {
    loop {
        let proceed = ask_user_input(&format!("{msg} [yn]"));
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
}
