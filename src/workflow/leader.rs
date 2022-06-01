// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::{client, io_msg::AppendTermResult, Url},
    common::error::ErrorResult,
    log_entry::Entries,
    node::Node,
    state::StatusPtr,
    Hook,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use tokio::sync::{Mutex, RwLock};
use tracing::{trace, warn};

/// Local enum used to trace how `post_new_append_term` worked
/// See also `Node::manage_append_term_result`
enum ReactResult {
    /// Retry the latest call
    Retry,
    /// Break the call loop
    Break,
    /// Continue th call loop
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
    /// uptodate.
    ///
    /// Look at the Raft documentation for more information.
    pub async fn run_leader(&self) -> ErrorResult<()> {
        self.start_loop_term_preparation();
        loop {
            if !self.p_status._is_leader().await {
                break;
            }
            let send_term_period = self.settings.get_send_term_sleep_duration();
            if matches!(self.internal_run_leader().await?, ReactResult::Break) {
                break;
            }
            let sleep = tokio::time::sleep(send_term_period);
            tokio::pin!(sleep);
            tokio::select! {
                _ = sleep => {}
                _ = tokio::signal::ctrl_c() => break,
            };
        }
        Ok(())
    }

    /// Send methods called in the leader send loop.
    ///
    /// Sends new terms for each nodes in the network
    async fn internal_run_leader(&self) -> ErrorResult<ReactResult> {
        self.increment_commit_term().await;
        let mut nodes = self.node_list.read().await.clone();
        nodes.extend(self.follower_list.read().await.clone());
        if nodes.is_empty() {
            let logs = self.logs.lock().await;
            if let Some(term) = logs.back() {
                if let Some(u) = term.parse_conn() {
                    nodes.insert(u.addr);
                }
            }
        }
        // todo: try to randomize the nodes list. We want to test with
        //       measurements if it's more efficients with a lot of nodes.
        //       (need to define a test case before!!)
        for node in nodes.iter() {
            if let ReactResult::Break =
                self.post_new_append_term(node.into()).await?
            {
                return Ok(ReactResult::Break);
            }
        }
        Ok(ReactResult::Continue)
    }

    /// Compute and post a new [AppendTermInput](crate::api::io_msg::AppendTermInput)
    /// to the `target`. Manage internaly the result.
    /// See `manage_append_term_result`.
    async fn post_new_append_term(
        &self,
        target: Url,
    ) -> ErrorResult<ReactResult> {
        let mut retry = true;
        loop {
            let url = target.clone();
            let append_term_input = self.create_term_input(&url).await;
            match client::post_append_term(
                &url,
                &self.settings,
                append_term_input,
            )
            .await
            {
                Ok(result) => {
                    let react =
                        self.manage_append_term_result(url, result).await?;
                    match react {
                        ReactResult::Retry => {
                            warn!("retry call to {}", target);
                            if retry {
                                retry = false;
                            } else {
                                warn!("failed multiple call to {}", target);
                                return Ok(ReactResult::Continue);
                            }
                        }
                        ReactResult::Break => return Ok(ReactResult::Break),
                        ReactResult::Continue => {
                            return Ok(ReactResult::Continue)
                        }
                    }
                }
                Err(p_warn) => {
                    warn!("{}, remove node from our index", *p_warn);
                    let mut a = self.node_list.write().await;
                    a.remove(&target.to_string());
                    let mut a = self.follower_list.write().await;
                    a.remove(&target.to_string());
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
    /// - continue means we're ok, send to next node.
    /// - retry means that we should retry to send an append_term message to
    ///   the node.
    /// - break means that we're now a follower, break all previous loops and
    ///   return in the main loop in `Node::start`.
    async fn manage_append_term_result(
        &self,
        target: Url,
        result: AppendTermResult,
    ) -> ErrorResult<ReactResult> {
        {
            let mut next_indexes_guard = self.next_indexes.write().await;
            if result.current_term > *self.p_current_term.lock().await {
                trace!(
                    "{} became leader with term {}",
                    target,
                    result.current_term.id
                );
                self.set_status_to_follower(target.to_string()).await?;
                return Ok(ReactResult::Break);
            }
            next_indexes_guard.insert(target.clone(), result.current_term.id);
        }
        if result.success {
            trace!("successfuly sent term to {}", target);
            Ok(ReactResult::Continue)
        } else {
            trace!("retry to send to {}", target);
            Ok(ReactResult::Retry)
        }
    }

    async fn increment_commit_term(&self) {
        let mut rates = HashMap::<usize, usize>::new();
        let mut max = 0;
        let mut max_v = 0;
        // todo: put that percent value in the setting file
        // todo: assert if 0 in node creation
        let percent = 55;
        // todo: allow user to be explicit (consider also follower or waiting
        //       nodes in tt_len)
        let nodes = self.node_list.read().await;
        let len = nodes.len();
        if nodes.is_empty() {
            std::mem::drop(nodes);
            let latest = self.logs.lock().await.back().unwrap().id;
            let mut index = self.p_commit_index.lock().await;
            if latest != *index {
                trace!("update commited index to {} by default", latest);
                self.commit_entries(*index, latest).await;
                *index = latest;
            }
            return;
        }
        for (url, next_index) in self.next_indexes.read().await.iter() {
            if !nodes.contains(&url.to_string()) {
                continue;
            }
            let n = if let Some(v) = rates.get_mut(next_index) {
                *v += 1;
                *v
            } else {
                rates.insert(*next_index, 1);
                1
            };
            if n > max_v || n == max_v && max < *next_index {
                max = *next_index;
                max_v = n;
            }
        }
        std::mem::drop(nodes);
        let mut index = self.p_commit_index.lock().await;
        trace!("better rated term {} scored {}", max, max_v);
        if max_v > len * percent / 100 && max > *index {
            trace!("update commited index to {} thanks to majority", max);
            self.commit_entries(*index, max).await;
            *index = max;
        }
    }

    /// Start a loop that prepare terms in parrallel. Fill the local `logs`
    /// parameter of the node
    fn start_loop_term_preparation(&self) {
        let p_logs = self.logs.clone();
        let p_status = self.p_status.clone();
        let prep_term_period = self.settings.get_prepare_term_sleep_duration();
        let waiting_nodes = self.waiting_nodes.clone();
        let hook = self.hook.clone();
        let nodes = self.node_list.clone();
        let followers = self.follower_list.clone();
        // todo: remove unwraps and handle errors
        tokio::spawn(async move {
            loop {
                let should_break = internal_term_preparation(
                    &p_logs,
                    &p_status,
                    &waiting_nodes,
                    &nodes,
                    &followers,
                    &hook,
                )
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
    p_status: &StatusPtr,
    waiting_nodes: &Arc<Mutex<VecDeque<String>>>,
    nodes: &Arc<RwLock<HashSet<String>>>,
    followers: &Arc<RwLock<HashSet<String>>>,
    hook: &Arc<Box<dyn Hook>>,
) -> bool {
    internal_term_preparation(
        p_logs,
        p_status,
        waiting_nodes,
        nodes,
        followers,
        hook,
    )
    .await
}

async fn internal_term_preparation(
    p_logs: &Arc<Mutex<Entries>>,
    p_status: &StatusPtr,
    waiting_nodes: &Arc<Mutex<VecDeque<String>>>,
    nodes: &Arc<RwLock<HashSet<String>>>,
    followers: &Arc<RwLock<HashSet<String>>>,
    hook: &Arc<Box<dyn Hook>>,
) -> bool {
    if !p_status._is_leader().await {
        // prepare term only if we are a Leader
        return true;
    }
    if nodes.read().await.is_empty() && followers.read().await.is_empty() {
        // prepare term only if there is someone listening :-)
        let mut waiting_nodes_guard = waiting_nodes.lock().await;
        if !waiting_nodes_guard.is_empty() {
            // create a term for the waiting node ;-)
            trace!("starter connect term");
            p_logs
                .lock()
                .await
                .append(conn_term_preparation(&mut waiting_nodes_guard, hook));
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
    p_logs.lock().await.append(term_content);
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
