//! todo: replace the temporary default with a better one
//!       (should be designed in another repository, using hook
//!       as a library)

use super::hook_trait::Hook;
use crate::log_entry::Term;
use std::{env, fs, process::Command};

pub(crate) struct DefaultHook;
impl Hook for DefaultHook {
    fn update_node(&self) -> bool {
        update_node()
    }

    fn pre_append_term(&self, _term: &Term) -> bool {
        pre_append_term(_term)
    }

    fn append_term(&self, _term: &Term) -> bool {
        append_term(_term)
    }

    fn commit_term(&self, _term: &Term) -> bool {
        commit_term(_term)
    }

    fn prepare_term(&self) -> String {
        prepare_term()
    }
}

fn get_script_path(prefix: &'static str) -> Option<String> {
    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        _ => return None,
    };
    for entry in fs::read_dir(current_dir).unwrap().flatten() {
        let path = entry.path();
        let path = path.to_str().unwrap();
        if path.starts_with(prefix) && !path.ends_with(&".sample") {
            return Some(path.to_string());
        }
    }
    None
}

fn update_node() -> bool {
    match get_script_path("update_node") {
        Some(script) => {
            let mut cmd = Command::new(script);
            let output = cmd.spawn().unwrap().wait_with_output().unwrap();
            let res = String::from_utf8(output.stdout).unwrap().to_lowercase();
            res != "false"
        }
        None => true,
    }
}

// todo, pass in arguments term and other flavours
fn pre_append_term(_term: &Term) -> bool {
    match get_script_path("pre_append_term") {
        Some(script) => {
            let mut cmd = Command::new(script);
            let output = cmd.spawn().unwrap().wait_with_output().unwrap();
            let res = String::from_utf8(output.stdout).unwrap().to_lowercase();
            res != "false"
        }
        None => true,
    }
}

// todo, pass in arguments term and other flavours
fn append_term(_term: &Term) -> bool {
    match get_script_path("append_term") {
        Some(script) => {
            let mut cmd = Command::new(script);
            let output = cmd.spawn().unwrap().wait_with_output().unwrap();
            let res = String::from_utf8(output.stdout).unwrap().to_lowercase();
            res != "false"
        }
        None => true,
    }
}

// todo, pass in arguments term and other flavours
fn commit_term(_term: &Term) -> bool {
    match get_script_path("commit_term") {
        Some(script) => {
            let mut cmd = Command::new(script);
            let output = cmd.spawn().unwrap().wait_with_output().unwrap();
            let res = String::from_utf8(output.stdout).unwrap().to_lowercase();
            res != "false"
        }
        None => true,
    }
}

// todo, pass in arguments term and other flavours
fn prepare_term() -> String {
    match get_script_path("prepare_term") {
        Some(script) => {
            let mut cmd = Command::new(script);
            let output = cmd.spawn().unwrap().wait_with_output().unwrap();
            String::from_utf8(output.stdout).unwrap().to_lowercase()
        }
        None => String::new(),
    }
}
