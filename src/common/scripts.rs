//! todo: replace the temporary default with a better one
//!       (should be designed in another repository, using hook
//!       as a library)

use super::hook_trait::Hook;
use crate::{log_entry::Term, state::EStatus};
use chrono::{SecondsFormat, Utc};
use serde::Deserialize;
use std::{env, fs, process::Command};
use tracing::{debug, warn};

pub struct DefaultHook;
impl Hook for DefaultHook {
    fn update_node(&self) -> bool {
        update_node()
    }

    fn pre_append_term(&self, _term: &Term) -> Option<usize> {
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

    fn retreive_term(&self, index: usize) -> Option<Term> {
        retreive_term(index)
    }

    fn retreive_terms(&self, from: usize, to: usize) -> Option<Vec<Term>> {
        retreive_terms(from, to)
    }

    fn switch_status(&self, status: EStatus) {
        switch_status(status)
    }
}

fn get_script_path(prefix: &'static str) -> Option<String> {
    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        _ => {
            debug!("no {prefix} found script");
            return None;
        }
    };
    for entry in fs::read_dir(current_dir).unwrap().flatten() {
        let name = entry.file_name();
        let name = name.to_str().unwrap();
        if name.starts_with(prefix) && !name.ends_with(&".sample") {
            return Some(entry.path().to_str().unwrap().to_string());
        }
    }
    debug!("no {prefix} found script");
    None
}

fn exec_cmd(script: String, input: Option<Vec<String>>) -> Option<String> {
    let mut cmd = Command::new(script.clone());
    debug!("exec script {} with args: {:?}", script, input);
    if let Some(args) = input {
        cmd.args(args);
    }
    let output = cmd.output().expect("failed reading output");
    debug!("{}: {:#?}", script, output);
    Some(String::from_utf8(output.stdout).unwrap().to_lowercase())
}

fn update_node() -> bool {
    match get_script_path("update_node") {
        Some(script) => {
            if let Some(res) = exec_cmd(script, None) {
                res == "true"
            } else {
                false
            }
        }
        None => true,
    }
}

fn pre_append_term(term: &Term) -> Option<usize> {
    let script = if let Some(script) = get_script_path("pre_append_term") {
        script
    } else {
        return Some(term.id);
    };
    let res = exec_cmd(
        script,
        Some(vec![format!("{}", term.id), term.content.clone()]),
    )?;
    if res.is_empty() {
        None
    } else {
        Some(res.parse().expect("Failed to parse pre_append_term output"))
    }
}

fn append_term(term: &Term) -> bool {
    match get_script_path("append_term") {
        Some(script) => {
            if let Some(res) = exec_cmd(
                script,
                Some(vec![format!("{}", term.id), term.content.clone()]),
            ) {
                res == "true"
            } else {
                false
            }
        }
        None => true,
    }
}

fn commit_term(term: &Term) -> bool {
    match get_script_path("commit_term") {
        Some(script) => {
            if let Some(res) = exec_cmd(
                script,
                Some(vec![format!("{}", term.id), term.content.clone()]),
            ) {
                res == "true"
            } else {
                false
            }
        }
        None => true,
    }
}

fn prepare_term() -> String {
    if let Some(script) = get_script_path("prepare_term") {
        if let Some(output) = exec_cmd(script, None) {
            return output;
        }
    }
    "default".into()
}

fn retreive_term(index: usize) -> Option<Term> {
    debug!("call retrieve term script");
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, false);
    let script = if let Some(script) = get_script_path("retrieve_term") {
        script
    } else {
        return Some(Term {
            id: index,
            timestamp,
            content: "default".into(),
        });
    };
    let content = exec_cmd(script, Some(vec![format!("{index}")]))?;
    debug!("content retrieved {} {}", index, content);
    Some(Term {
        id: index,
        timestamp,
        content,
    })
}

fn retreive_terms(from: usize, to: usize) -> Option<Vec<Term>> {
    debug!("call retrieve termS script");
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, false);
    let script = get_script_path("retrieve_n_term");
    if script.is_none() {
        return Some(
            (from..=to)
                .map(|id| Term {
                    id,
                    timestamp: timestamp.clone(),
                    content: "default".into(),
                })
                .collect(),
        );
    }

    let output = exec_cmd(
        script.unwrap(),
        Some(vec![format!("{from}"), format!("{to}")]),
    )?;

    #[derive(Deserialize)]
    struct TermWithoutTimestamp {
        id: usize,
        content: String,
    }

    match serde_json::from_str::<Vec<TermWithoutTimestamp>>(&output) {
        Ok(terms) => {
            debug!("parse retrieve termS succeed");
            Some(
                terms
                    .into_iter()
                    .map(|term| Term {
                        id: term.id,
                        timestamp: timestamp.clone(),
                        content: term.content,
                    })
                    .collect(),
            )
        }
        Err(err) => {
            warn!("{:?} failed to parse retrieve termS output {}", err, output);
            None
        }
    }
}

fn switch_status(status: EStatus) {
    if let Some(script) = get_script_path("switch_status") {
        exec_cmd(
            script,
            Some(vec![match status {
                EStatus::ConnectionPending => "pending",
                EStatus::Follower => "follower",
                EStatus::Leader => "leader",
                EStatus::Candidate => "candidate",
            }
            .to_string()]),
        );
    }
}
