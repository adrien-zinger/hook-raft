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
    node::Node,
};
use tracing::{debug, trace, trace_span, warn};

macro_rules! log {
    ($($rest:tt)*) => {
        #[cfg(not(test))]
        trace!($($rest)*);
        #[cfg(test)]
        println!($($rest)*);
    }
}

impl Node {
    /// Reception of a append_term request.
    ///
    /// - check inputs, if it's enough updated, if we have the previous term.
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
        debug!("received new term {:#?}", input,);
        self.internal_receive_append_term(input).await
    }

    /// internal implementation of receive append term
    ///
    /// Specification
    /// 1. Reply false if term < currentTerm (§5.1)
    /// 2. Reply false if log doesn’t contain an entry at prevLogIndex
    /// whose term matches prevLogTerm (§5.3)
    /// 3. If an existing entry conflicts with a new one (same index
    /// but different terms), delete the existing entry and all that
    /// follow it (§5.3)
    /// 4. Append any new entries not already in the log
    /// 5. If leaderCommit > commitIndex, set commitIndex =
    /// min(leaderCommit, index of last new entry
    async fn internal_receive_append_term(
        &self,
        mut input: AppendTermInput,
    ) -> ErrorResult<AppendTermResult> {
        input.entries.sort_by_key(|e| e.id);

        let checks = self.check_input(&input).await;
        if let Err(res) = checks {
            log!("request rejected by checks");
            return Ok(res);
        }
        self.reset_timeout().await;

        if input.prev_term.id == 1 {
            trace!("pre/append root term");
            if let Some(index) = self.hook.pre_append_term(&input.prev_term) {
                if index < input.prev_term.id {
                    log!("root term rejected by checks pre append term");
                    return Ok(AppendTermResult {
                        current_term: self.logs.lock().await.current_term(),
                        success: false,
                    });
                }
                self.logs.lock().await.insert(&input.prev_term);
                self.hook.append_term(&input.prev_term);
            } else {
                panic!("request rejected in pre append term");
            }
        } else {
            // Pre/Append entries (from prev term to term excluded)
            if !input.entries.is_empty() {
                for term in &input.entries {
                    if term.id <= self.logs.lock().await.commit_index() {
                        // ignore committed term
                        continue;
                    }
                    // pre append term send the last index I don't have
                    // (the first missing term).
                    //
                    // - if term from input == last I don't have => OK
                    // - return that index - 1 (the last I have / current term) otherwise
                    if let Some(index) = self.hook.pre_append_term(term) {
                        if index < term.id {
                            trace!(
                                "term (entries) {} rejected by checks pre append term",
                                index
                            );
                            return Ok(AppendTermResult {
                                current_term: self.logs.lock().await.current_term(),
                                success: false,
                            });
                        }
                        self.logs.lock().await.insert(term);
                        self.hook.append_term(term);
                    } else {
                        // todo: throw an internal error
                        panic!("request rejected in pre append term");
                    }
                }
            }

            if let Some(index) = self.hook.pre_append_term(&input.term) {
                if index < input.term.id {
                    log!("term {} rejected by checks pre append term", index);
                    return Ok(AppendTermResult {
                        current_term: self.logs.lock().await.current_term(),
                        success: false,
                    });
                }
                self.logs.lock().await.insert(&input.term);
                self.hook.append_term(&input.term);
            } else {
                panic!("request rejected in pre append term");
            }
        }
        log!("request {} has passed checks", input.term.id);

        // Finally commit the entries up to leader_commit_index,
        // stopping at the latest entry we have in cache
        self.commit_entries(input.leader_commit_index).await;

        // Since leader care about that the last term (term in the input)
        // is the biggest term we should know at the end of AppendEntries.
        // We can be almost sure that current term IS the input term here.
        let current_term = self.logs.lock().await.current_term();
        let current_term_id = current_term.id;

        trace!("append term success. latest term: {:#?}", current_term);
        Ok(AppendTermResult {
            current_term,
            success: current_term_id <= input.term.id,
        })
    }

    /// Check if the current term is at least equals to the term local.
    ///
    /// Also send an error and inform the leader about his last current_term
    /// to adapt his own `next_indexes` table and ensure the logs consistency
    /// in the next call.
    async fn check_input(&self, input: &AppendTermInput) -> Result<(), AppendTermResult> {
        let mut logs_guard = self.logs.lock().await;
        let current_term = logs_guard.current_term();
        if input.term.id < current_term.id {
            log!("term id older than local state");
            return Err(AppendTermResult {
                current_term,
                success: false,
            });
        }

        if input.leader_commit_index < logs_guard.commit_index() {
            log!("leader commit index invalid");
            return Err(AppendTermResult {
                current_term,
                success: false,
            });
        }

        // We need the message to have all the entries between `term`
        // and `prev_term`.
        if input.prev_term.id > input.term.id {
            return Err(AppendTermResult {
                current_term,
                success: false,
            });
        }
        if input.term.id == input.prev_term.id && input.entries.is_empty()
            || input.entries.len() == input.term.id - input.prev_term.id - 1
        {
            // Input entries contains a correct number of terms. Now
            // we check if terms ids are monotones.
            let ids = input.prev_term.id + 1..=input.term.id;
            for (e, expected_id) in input.entries.iter().zip(ids) {
                if e.id != expected_id {
                    log!("entry missing, jump from {} to {}", e.id, expected_id);
                    return Err(AppendTermResult {
                        current_term,
                        success: false,
                    });
                }
            }
        } else {
            log!(
                "entries received are len {}, and we expected {} - {} = {}",
                input.entries.len(),
                input.term.id,
                input.prev_term.id,
                input.term.id - input.prev_term.id
            );
            return Err(AppendTermResult {
                current_term,
                success: false,
            });
        }

        // 2. Reply false if log doesn’t contain an entry at prevLogIndex
        // whose term matches prevLogTerm (§5.3)
        if let Some(local_term) = logs_guard.find(input.prev_term.id) {
            // found in cache
            if local_term != input.prev_term {
                // 3. If an existing entry conflicts with a new one (same index
                // but different terms), delete the existing entry and all that
                // follow it (§5.3)
                logs_guard.insert(&input.prev_term);
                self.hook.append_term(&input.prev_term);
            }
        } else if input.prev_term.id <= logs_guard.commit_index() {
            // has been committed
            // todo: add a hook here like "check terms validity" to verify
            // if it match correctly with local terms
        } else if input.prev_term.id == 1 {
            // its also OK to receive a root term once.
            // todo: accept once
        } else {
            warn!("unable to find the previous term");
            return Err(AppendTermResult {
                current_term,
                success: false,
            });
        }

        let _ = self
            .switch_to_follower(input.leader_id.clone().into())
            .await;
        Ok(())
    }
}
