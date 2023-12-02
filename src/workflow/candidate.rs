// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

//! Also called the heartbeat timeout, when this timeout reach, we start a
//! candidature workflow.
//!
//! The file contain the candidature implementation. When a node stop to be a
//! follower, he increment his own logs and send `request_votes` request to
//! the other potential candidates in the network.
//!
//! - if majority reached, start to be a leader
//! - if alone in the network, start to be a leader
//!
//! When the candidature process finish, if the node is still a candidate, wait
//! a random time and restart a candidature.
//!
//! If candidate is a leader at the end of the workflow, start the leader
//! workflow. [crate::workflow::leader]
//!
//! If candidate has received a new term better from a new leader
//! [crate::workflow::append_term], at the end of the candidature, start the
//! follower workflow [crate::workflow::follower].

use crate::{
    api::{client, io_msg::RequestVoteInput, Url},
    common::error::ErrorResult,
    log_entry::LogEntry,
    node::Node,
    Settings,
};
use tracing::{debug, trace, warn};

impl Node {
    /// - On conversion to candidate, start election:
    /// - Increment currentTerm
    /// - Vote for self
    /// - Reset election timer
    /// - Send RequestVote RPCs to all other servers
    /// - If votes received from majority of servers: become leader
    /// - If AppendEntries RPC received from new leader: convert to
    ///   follower
    /// - If election timeout elapses: start new election
    pub async fn run_candidate(&self) -> ErrorResult<()> {
        let (commit_index, mut last_term) = {
            let mut logs = self.logs.lock().await;
            let commit_index = logs.last_index(); // todo: wrong names
            if let Some((_, id)) = &*self.vote_for.read().await {
                if commit_index <= *id {
                    return Ok(());
                }
            }
            (commit_index, logs.append("candidature".into()))
        };

        self.next_indexes.write().await.clear();
        self.hook.append_term(&last_term);
        *self.vote_for.write().await = Some((self.settings.node_id.clone(), commit_index));

        while self
            .start_candidature(commit_index, last_term.clone())
            .await
        {
            last_term = self.logs.lock().await.append("candidature".into());
            self.hook.append_term(&last_term);
            *self.vote_for.write().await = Some((self.settings.node_id.clone(), commit_index));
        }

        Ok(())
    }

    async fn start_candidature(&self, commit_index: usize, last_term: LogEntry) -> bool {
        let res = self.async_calls_candidature(commit_index, last_term).await;
        // clean vote
        *self.vote_for.write().await = None;

        trace!("candidature finished!");
        if res {
            self.switch_to_leader().await.unwrap();
            return false;
        }
        if self.p_status.is_follower().await {
            return false;
        }
        let dur = self.settings.get_randomized_timeout();
        debug!("wait {:?} before a new timeout", dur);
        tokio::time::sleep(dur).await;
        self.p_status.is_candidate().await
    }

    async fn async_calls_candidature(&self, commit_index: usize, last_term: LogEntry) -> bool {
        let nodes = self.node_list.read().await.clone();
        if nodes.is_empty() {
            trace!("no other nodes, will turn into a leader by default");
            return true;
        }
        let len = nodes.len();
        let mut granted_vote_count = 0;
        for node in nodes {
            if self.p_status.is_follower().await {
                return false;
            }
            // todo: manage all warning, (return an error and stop the node)
            call_candidature(
                &node.into(),
                &self.settings,
                &last_term,
                commit_index,
                &mut granted_vote_count,
            )
            .await
        }

        trace!("candidature finished with score {}", granted_vote_count);
        if let Some((vote, _)) = &*self.vote_for.read().await {
            if *vote == format!("{}:{}", self.settings.addr, self.settings.port) {
                return granted_vote_count + 1 > (len / 2); // todo use quorum from settings
            }
        }
        granted_vote_count > (len / 2) // todo use quorum from settings
    }
}

/// Make the post request and manage a unitary result
async fn call_candidature(
    target: &Url,
    settings: &Settings,
    last_term: &LogEntry,
    commit_index: usize,
    granted_vote_count: &mut usize,
) {
    // todo: we may want in case of fail make a hook
    match client::post_request_vote(
        target,
        settings,
        RequestVoteInput {
            candidate_id: settings.node_id.clone(),
            term: last_term.clone(),
            last_term: commit_index,
        },
    )
    .await
    {
        Ok(res) => {
            debug!("vote request response received {:#?}", res);
            if res.vote_granted {
                *granted_vote_count += 1;
            }
        }
        Err(err) => {
            warn!(
                "failed to request vote to {},\n{:indent$}",
                target,
                *err,
                indent = 2
            );
        }
    }
}
