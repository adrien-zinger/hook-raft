// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.
//

//! # APPEND TERM
//!
//! Implementation of the workflow when node receive a `append_term` request.

use crate::{
    api::io_msg::{AppendTermInput, AppendTermResult},
    common::error::ErrorResult,
    log_entry::Term,
    node::Node,
};
use std::cmp::min;
use tracing::{debug, trace, trace_span};

impl Node {
    // todo: a new incommers should be able to connect with a prepared
    //       list of logs. (Previous term and commit index inside the settings?
    //       or a memmapped file? or a hook?)
    //       The leader can be able to reject a really
    //       unsynchronized node.

    /// Reception of a append_term request.
    ///
    /// - check inputs, if it's enough uptodate, if we have the previous term.
    /// - increment the heartbeat timeout
    /// - call hook pre_append_term
    /// - set status to follower if input term > local term
    /// - update local log entries, commit if commit index updated
    pub async fn receive_append_term(
        &self,
        input: AppendTermInput,
    ) -> ErrorResult<AppendTermResult> {
        let span = trace_span!("receive_append_term");
        let _enter = span.enter();
        trace!(
            "received new term {}: '{}'",
            input.term.id,
            input.term.content,
        );
        debug!(
            "received new term {}, prev {}, entries size {}",
            input.term.id,
            input.prev_term.id,
            input.entries.len()
        );
        self.internal_receive_append_term(input).await
    }

    /// internal implementation of receive append term
    async fn internal_receive_append_term(
        &self,
        input: AppendTermInput,
    ) -> ErrorResult<AppendTermResult> {
        let mut current_term = self.p_current_term.lock().await.clone();
        let checks = self.check_input(&input, &current_term).await;
        if let Err(res) = checks {
            trace!("request rejected by checks");
            return Ok(res);
        }
        self.increment_heartbeat_timeout().await;
        if !self.hook.pre_append_term(&input.term) {
            trace!("request rejected by script");
            return Ok(AppendTermResult {
                current_term,
                success: false,
            });
        }
        trace!("request {} has passed checks", input.term.id);
        if input.term.id > current_term.id {
            // If RPC request or response contains term T > currentTerm:
            // set currentTerm = T, convert to follower
            trace!("_input.term.id > current_term.id_");
            self.set_status_to_follower(input.leader_id.clone()).await?;
        }
        self.update_local_entries(&input).await;
        self.replace_latest_if_necessary(&input, &mut current_term)
            .await;
        *self.p_current_term.lock().await = current_term.clone();
        Ok(AppendTermResult {
            current_term,
            success: true,
        })
    }

    /// Update the local entries if there is a diff. And commit the entry if
    /// the commit index is updated.
    async fn update_local_entries(&self, input: &AppendTermInput) {
        let mut latest_id = self.append_entries(input).await;
        let mut index_guard = self.p_commit_index.lock().await;
        if input.leader_commit_index > *index_guard {
            latest_id = min(input.leader_commit_index, latest_id);
            self.commit_entries(*index_guard, latest_id).await;
            *index_guard = latest_id;
        }
    }

    /// Append all entries in the local logs. If we found an entry with a
    /// conflict, we remove all the successors (done in the insert function)
    ///
    /// - If two entries in different logs have the same index
    /// and term, then they store the same command.
    /// - If two entries in different logs have the same index
    /// and term, then the logs are identical in all preceding
    /// entries.
    async fn append_entries(&self, input: &AppendTermInput) -> usize {
        let mut a = input.entries.iter().collect::<Vec<_>>();
        a.sort_by_key(|t| t.id);
        let mut logs = self.logs.lock().await;
        for entry in a.iter() {
            trace!("append entry {}", entry.id);
            if logs.insert(entry) {
                self.hook.append_term(entry);
            }
        }
        logs.insert(&input.term);
        trace!("append entry {}", input.term.id);
        self.hook.append_term(&input.term);
        input.term.id
    }

    /// If local current term is different, replace it with the input.
    async fn replace_latest_if_necessary(
        &self,
        input: &AppendTermInput,
        current_term: &mut Term,
    ) {
        if input.term.id != current_term.id
            || input.term.content != current_term.content
        {
            trace!(
                "update local term from {} to {}",
                current_term.id,
                input.term.id
            );
            *current_term = input.term.clone();
        }
    }

    /// Check if the current term is at least equals to the term local.
    ///
    /// Also send an error and inform the leader about his last current_term
    /// to adapt his own `next_indexes` table and ensure the logs consistency
    /// in the next call.
    async fn check_input(
        &self,
        input: &AppendTermInput,
        current_term: &Term,
    ) -> Result<(), AppendTermResult> {
        if input.term.id < current_term.id {
            trace!("term id older than local state");
            return Err(AppendTermResult {
                current_term: current_term.clone(),
                success: false,
            });
        }
        let mut logs_guard = self.logs.lock().await;
        if self.p_status._is_pending().await {
            trace!("append entry {}", input.prev_term.id);
            logs_guard.insert(&input.prev_term);
            return Ok(());
            // todo: we can write a setting that allow user to force to be hardly
            //       synchronized for the connection. In that case, just do the
            //       next check and skip this one.
        }
        if !logs_guard.contains(&input.prev_term) {
            trace!("unknow previous term of the request");
            return Err(AppendTermResult {
                current_term: current_term.clone(),
                success: false,
            });
        }
        Ok(())
    }

    /// Increment the heartbeat timeout if it's initialized
    async fn increment_heartbeat_timeout(&self) {
        // todo: in the dyn timeout crate, add a max value to wait
        let opt_heartbeat_guard = self.opt_heartbeat.lock().await;
        if opt_heartbeat_guard.is_none() {
            trace!("heartbeat not initialized");
            return;
        }
        trace!("increment heartbeat timeout");
        if let Some(heartbeat) = &*opt_heartbeat_guard {
            heartbeat
                .add(self.settings.get_randomized_inc_timeout())
                .await
                .unwrap(); // todo: add from settings, remove unwraps (and dismiss lib panic)
        }
    }
}
