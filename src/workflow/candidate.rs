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
//! a random time and restat a candidature.
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
use tracing::{trace, warn};

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
        let (last_term, term) = {
            let mut logs = self.logs.lock().await;
            (logs.back().unwrap(), logs.append(String::new()))
        };

        {
            *self.p_current_term.lock().await = term.clone();
            *self.vote_for.write().await = Some((
                format!("{}:{}", self.settings.addr, self.settings.port),
                last_term.clone(),
            ));
        }
        while self
            .start_candidature(last_term.clone(), term.clone())
            .await
        {}
        Ok(())
    }

    async fn start_candidature(
        &self,
        last_term: LogEntry,
        term: LogEntry,
    ) -> bool {
        let res = self.async_calls_candidature(last_term, term).await;
        trace!("candidature finished!");
        if res {
            self.set_status_leader().await.unwrap();
            return false;
        }
        let dur = self.settings.get_randomized_timeout();
        let p_st = self.p_status.clone();
        tokio::time::sleep(dur).await;
        p_st._is_candidate().await && !res
    }

    async fn async_calls_candidature(
        &self,
        last_term: LogEntry,
        term: LogEntry,
    ) -> bool {
        let nodes = self.node_list.read().await.clone();
        if nodes.is_empty() {
            trace!("no other nodes, will turn into a leader by default");
            return true;
        }
        let len = nodes.len();
        let mut granted_vote_count = 0;
        for node in nodes {
            // todo: manage all warning, (return an error and stop the node)
            call_candidature(
                &node.into(),
                &self.settings,
                &term,
                &last_term,
                &mut granted_vote_count,
            )
            .await
        }
        let r = granted_vote_count as f64 / len as f64;
        trace!("candidature finished with score {}", r);
        r >= 0.5 // todo: make that test in settings or in a hook
    }
}

/// Make the post request and manage a unitary result
async fn call_candidature(
    target: &Url,
    settings: &Settings,
    term: &LogEntry,
    last_term: &LogEntry,
    granted_vote_count: &mut usize,
) {
    // todo: we may want in case of fail make a hook
    match client::post_request_vote(
        target,
        settings,
        RequestVoteInput {
            candidate_id: format!("{}:{}", settings.addr, settings.port),
            term: term.clone(),
            last_term: last_term.clone(),
        },
    )
    .await
    {
        Ok(res) => {
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
