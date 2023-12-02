// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::{client, io_msg::AppendTermResult, Url},
    common::error::ErrorResult,
    log_entry::{Entries, Term},
    node::Node,
    state::Status,
    Hook,
};
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, trace, warn};

/// Local enum used to trace how `post_new_append_term` worked
/// See also `Node::manage_append_term_result`
enum ReactResult {
    /// Retry the latest call
    Retry,
    /// Break the call loop
    Break,
    /// Continue the call loop
    Continue,
}

impl Node {
    /// Start the leader workflow.
    ///
    /// - Run the term preparation loop that create a terms with a frequency of
    ///   prepare_term_period.
    /// - Run a loop to send a new term to other nodes each `send_term_period`
    ///
    /// Run the loops until someone else take the lead or handle ctrl_c.
    /// Leader understand if someone took the lead if another node is more
    /// updated.
    ///
    /// Look at the Raft documentation for more information.
    pub async fn run_leader(&self) -> ErrorResult<()> {
        self.start_loop_term_preparation();
        loop {
            if !self.p_status.is_leader().await {
                trace!("stop lead");
                break;
            }
            let _ = self.internal_run_leader().await?;
        }
        Ok(())
    }

    /// Send methods called in the leader send loop.
    ///
    /// Sends new terms for each nodes in the network
    async fn internal_run_leader(&self) -> ErrorResult<ReactResult> {
        let nodes = self.node_list.read().await.clone();
        let mut fail_count = 0;
        trace!("start a sending session as leader");
        for node in nodes.iter() {
            if let ReactResult::Break = self
                .post_new_append_term(node.into(), &mut fail_count)
                .await?
            {
                return Ok(ReactResult::Break);
            }
        }

        // Increment the commit term after the calls
        self.increment_commit_term().await;

        if fail_count > (self.node_list.read().await.len() / 2) {
            warn!("quorum is unreachable, switch to candidate");
            self.switch_to_candidate().await?;
            return Ok(ReactResult::Break);
        }

        Ok(ReactResult::Continue)
    }

    /// Compute and post a new [AppendTermInput](crate::api::io_msg::AppendTermInput)
    /// to the `target`. Manage internally the result.
    /// See `manage_append_term_result`.
    async fn post_new_append_term(
        &self,
        target: Url,
        fail_count: &mut usize,
    ) -> ErrorResult<ReactResult> {
        let mut retry = 100;
        loop {
            if !self.p_status.is_leader().await {
                return Ok(ReactResult::Break);
            }
            let url = target.clone();
            let append_term_input = self.create_term_input(&url).await;
            match client::post_append_term(&url, &self.settings, append_term_input).await {
                Ok(result) => {
                    let react = self.manage_append_term_result(url, result).await?;
                    match react {
                        ReactResult::Retry => {
                            warn!("retry call to {}", target);
                            retry -= 1;
                            if retry == 0 {
                                warn!("failed multiple call to {}", target);
                                return Ok(ReactResult::Continue);
                            }
                        }
                        ReactResult::Break => return Ok(ReactResult::Break),
                        ReactResult::Continue => return Ok(ReactResult::Continue),
                    }
                }
                Err(p_warn) => {
                    warn!("{}", *p_warn);
                    // Not sure if we want to ban node, call a hook instead
                    *fail_count += 1;
                    return Ok(ReactResult::Continue);
                }
            }
        }
    }

    /// Manage a result of a `post_append_term` call.
    ///
    /// If RPC request or response contains term T > currentTerm:
    /// set currentTerm = T, convert to follower.
    ///
    /// If last log index >= nextIndex for a follower: send
    /// append_term post request with log entries starting at nextIndex
    /// - If successful: update nextIndex and matchIndex for follower
    /// - If fails because of log inconsistency: use the last entry returned
    ///   by the distant node and retry.
    ///
    /// # Result
    ///
    /// Can return a [ReactResult]
    /// - continue means we're OK, send to next node.
    /// - retry means that we should retry to send an append_term message to
    ///   the node.
    /// - break means that we're now a follower, break all previous loops and
    ///   return in the main loop in `Node::start`.
    async fn manage_append_term_result(
        &self,
        target: Url,
        result: AppendTermResult,
    ) -> ErrorResult<ReactResult> {
        use crate::node::NextIndex::{Pending, Validated};

        {
            let mut next_indexes_guard = self.next_indexes.write().await;
            if result.current_term.id > self.logs.lock().await.last_index() {
                trace!(
                    "{target} became leader with term {}",
                    result.current_term.id
                );
                self.switch_to_follower(target).await?;
                return Ok(ReactResult::Break);
            }

            if result.current_term.id > 0 {
                if let Some(local_term) = self.leader_retreive_term(result.current_term.id).await {
                    if local_term == result.current_term {
                        debug!(
                            "node return a current term {:#?} validated",
                            result.current_term
                        );
                        next_indexes_guard
                            .insert(target.clone(), Validated(result.current_term.id));
                    } else {
                        debug!(
                            "node return a unmatched current term {:#?}",
                            result.current_term
                        );
                        next_indexes_guard
                            .insert(target.clone(), Pending(result.current_term.id - 1));
                    }
                } else {
                    warn!("leader can't find a term");
                }
            } else {
                next_indexes_guard.insert(target.clone(), Validated(1));
            }
        }

        if result.success {
            trace!("successfully sent term to {}", target);
            Ok(ReactResult::Continue)
        } else {
            trace!("retry to send to {} after {:#?}", target, result);
            Ok(ReactResult::Retry)
        }
    }

    async fn leader_retreive_term(&self, index: usize) -> Option<Term> {
        if let Some(term) = self.logs.lock().await.find(index) {
            return Some(term);
        }
        if let Some(term) = self.hook.retreive_term(index) {
            return Some(term);
        }
        None
    }

    async fn increment_commit_term(&self) {
        let nodes = self.node_list.read().await;
        let len = nodes.len();
        if nodes.is_empty() {
            trace!("pass commit phase with no nodes");
            return;
        }
        std::mem::drop(nodes);

        let mut votes = Vec::<(usize, usize)>::new();
        let mut set = HashSet::<usize>::new();

        for index in self
            .next_indexes
            .read()
            .await
            .iter()
            .map(|(_, v)| v.validated())
        {
            if !set.contains(&index) {
                set.insert(index);
                votes.push((index, 0));
                votes.sort_by_key(|(index, _)| *index);
            }
            for (i, n) in votes.iter_mut() {
                if *i <= index {
                    *n += 1
                } else {
                    break;
                }
            }
        }

        let mut max_term_id = 0;
        debug!("check latest commit: votes {:?}", votes);
        for (i, n) in votes {
            // todo: take quorum from settings
            if n >= (len / 2) {
                max_term_id = i;
            }
        }

        trace!("better rated term {max_term_id}");
        self.commit_entries(max_term_id).await;
    }

    /// Start a loop that prepare terms in parallel. Fill the local `logs`
    /// parameter of the node
    fn start_loop_term_preparation(&self) {
        let p_logs = self.logs.clone();
        let p_status = self.p_status.clone();
        let prep_term_period = self.settings.get_prepare_term_sleep_duration();
        let waiting_nodes = self.waiting_nodes.clone();
        let hook = self.hook.clone();
        let nodes = self.node_list.clone();
        // todo: remove unwraps and handle errors
        tokio::spawn(async move {
            loop {
                let should_break =
                    internal_term_preparation(&p_logs, &p_status, &waiting_nodes, &nodes, &hook)
                        .await;
                if should_break {
                    break;
                }
                let sleep = tokio::time::sleep(prep_term_period);
                tokio::pin!(sleep);
                tokio::select! {
                    _ = sleep => {}
                    _ = tokio::signal::ctrl_c() => break,
                };
            }
        });
    }
}

#[cfg(test)]
pub async fn _term_preparation(
    p_logs: &Arc<Mutex<Entries>>,
    p_status: &Status,
    waiting_nodes: &Arc<Mutex<VecDeque<String>>>,
    nodes: &Arc<RwLock<HashSet<String>>>,
    hook: &Arc<Box<dyn Hook>>,
) -> bool {
    internal_term_preparation(p_logs, p_status, waiting_nodes, nodes, hook).await
}

async fn internal_term_preparation(
    p_logs: &Arc<Mutex<Entries>>,
    p_status: &Status,
    waiting_nodes: &Arc<Mutex<VecDeque<String>>>,
    nodes: &Arc<RwLock<HashSet<String>>>,
    hook: &Arc<Box<dyn Hook>>,
) -> bool {
    if !p_status.is_leader().await {
        // prepare term only if we are a Leader
        return true;
    }
    if nodes.read().await.is_empty() {
        // prepare term only if there is someone listening :-)
        let mut waiting_nodes_guard = waiting_nodes.lock().await;
        if !waiting_nodes_guard.is_empty() {
            // create a term for the waiting node
            trace!("starter connect term");
            let term = p_logs
                .lock()
                .await
                .append(conn_term_preparation(&mut waiting_nodes_guard, hook));
            hook.append_term(&term);
        }
        return false;
    }
    trace!("start term preparation");
    let mut waiting_nodes_guard = waiting_nodes.lock().await;
    let term_content = if waiting_nodes_guard.is_empty() || rand::random() {
        trace!("hook term handling");
        hook.prepare_term()
    } else {
        conn_term_preparation(&mut waiting_nodes_guard, hook)
    };
    let term = p_logs.lock().await.append(term_content);
    hook.append_term(&term);
    false
}

fn conn_term_preparation(
    waiting_nodes: &mut VecDeque<String>,
    hook: &Arc<Box<dyn Hook>>,
) -> String {
    // todo: checkout multiple waiting nodes accordingly to
    //       some user settings to define
    let p = waiting_nodes.pop_front();
    if let Some(n) = p {
        trace!("creation of a connect term");
        format!("conn:{}", n)
    } else {
        warn!("unexpected hook term handling");
        hook.prepare_term()
    }
}
