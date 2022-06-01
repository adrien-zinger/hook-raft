// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

//! On receive request vote workflow. Interpretation of the description of
//! what a hook node does on receive a vote request. For more information, look
//! at the raft paper on that repository and the original repository
//! https://raft.github.io/.

use crate::{
    api::io_msg::{RequestVoteInput, RequestVoteResult},
    Node,
};

impl Node {
    /// Node reaction on receive a vote request.
    pub async fn receive_request_vote(
        &self,
        input: RequestVoteInput,
    ) -> RequestVoteResult {
        let current_term = self.p_current_term.lock().await.clone();
        // todo: hook receive request vote
        if input.term.id < current_term.id {
            return RequestVoteResult {
                current_term,
                vote_granted: false,
            };
        }
        let latest = self.logs.lock().await.back().unwrap();
        let mut opt_vote = self.vote_for.write().await;
        let vote_granted = if let Some(vote) = opt_vote.clone() {
            input.last_term.id > vote.1.id
        } else {
            input.last_term.id >= latest.id
        };
        if vote_granted {
            *opt_vote = Some((input.candidate_id, input.last_term))
        }
        RequestVoteResult {
            current_term,
            vote_granted,
        }
    }
}
