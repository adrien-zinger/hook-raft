use serde::{Deserialize, Serialize};

use crate::common::error::HttpErrorResult;
use crate::log_entry::Term;

#[derive(Debug, Deserialize, Serialize)]
pub enum HttpResult {
    RequestVote(RequestVoteResult),
    UpdateNode(UpdateNodeResult),
    AppendTerm(AppendTermResult),
    Error(HttpErrorResult),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestVoteInput {
    pub candidate_id: String,
    pub term: Term,
    // commit index
    pub last_term: usize,
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
    pub entries: Vec<Term>,
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
}

impl PartialEq for UpdateNodeInput {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateNodeResult {
    pub leader_id: String,
    pub node_list: Vec<String>,
}
