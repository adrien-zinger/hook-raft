use serde::{Deserialize, Serialize};

use crate::log_entry::{Entries, Term};

#[derive(Debug, Deserialize, Serialize)]
pub enum HttpResult {
    RequestVote(RequestVoteResult),
    UpdateNode(UpdateNodeResult),
    AppendTerm(AppendTermResult),
    Error(HttpErrorResult),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HttpErrorResult {
    pub err_id: String,
    pub message: String,
}

impl std::fmt::Display for HttpErrorResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "network error id: {}\n error message: {}",
            self.err_id, self.message
        ))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestVoteInput {
    pub candidate_id: String,
    pub term: Term,
    pub last_term: Term,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestVoteResult {
    pub current_term: Term,
    pub vote_granted: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppendTermInput {
    pub term: Term,
    pub leader_id: String,
    pub prev_term: Term,
    pub entries: Entries,
    pub leader_commit_index: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppendTermResult {
    pub current_term: Term,
    pub success: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateNodeInput {
    /// Unique identifier of the node
    pub hash: [u8; 16],
    /// Open server port
    pub port: String,
    /// Follower flag
    pub follower: bool,
}

impl PartialEq for UpdateNodeInput {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateNodeResult {
    pub leader_id: String,
    pub node_list: Vec<String>, // todo how is it with hashset here ?
    pub follower_list: Vec<String>,
}
