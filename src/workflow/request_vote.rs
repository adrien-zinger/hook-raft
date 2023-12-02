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

use tracing::{debug, trace};

impl Node {
    /// Node reaction on receive a vote request.
    pub async fn receive_request_vote(&self, input: RequestVoteInput) -> RequestVoteResult {
        trace!("receive a vote request {:#?}", input);
        let current_term = self.logs.lock().await.current_term();
        // todo: hook receive request vote
        if input.term.id < current_term.id {
            debug!("refuse candidates because term < current");
            return RequestVoteResult {
                current_term,
                vote_granted: false,
            };
        }
        let mut opt_vote = self.vote_for.write().await;

        let vote_granted = if let Some(vote) = opt_vote.clone() {
            debug!("compare to previous vote");
            input.last_term > vote.1 || input.candidate_id == vote.0
        } else {
            debug!("compare to previous vote");
            input.last_term >= self.logs.lock().await.commit_index()
        };

        debug!("vote granted: {vote_granted}");
        if vote_granted {
            *opt_vote = Some((input.candidate_id, input.last_term));
            self.reset_timeout().await
        }
        RequestVoteResult {
            current_term: self.logs.lock().await.current_term(),
            vote_granted,
        }
    }
}
