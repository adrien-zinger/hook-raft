// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::{io_msg::AppendTermInput, Url},
    log_entry::{Entries, Term},
    node::Node,
};
use tokio::sync::MutexGuard;
use tracing::debug;

impl Node {
    /// Get term for target node
    async fn get_target_term<'a>(
        &self,
        target_node: &Url,
        logs_guard: &mut MutexGuard<'a, Entries>,
    ) -> Term {
        // If a node needs a specific term, we try to find it in the logs,
        // otherwise we defer the job to the hook.
        //
        // If the node doesn't have a next_indexes registered, fill with the
        // current term.
        match self.next_indexes.read().await.get(target_node) {
            Some(node_index) => {
                let id = node_index.unwrap();
                match logs_guard.find(id) {
                    Some(term) => term,
                    _ => self
                        .hook
                        .retreive_term(id)
                        .expect("Unable to retrieve term {id} as leader"),
                }
            }
            _ => {
                // suppose the last term is in under the latest
                // leader commit. Minimum index is 1.
                let mut index = logs_guard.commit_index();
                index = index.saturating_sub(10);
                if index == 0 {
                    index = 1;
                }
                self.hook
                    .retreive_term(index)
                    .expect("Unable to retrieve term {id} as leader")
            }
        }
    }

    /// Creates a term especially for the `target_node`
    pub(crate) async fn create_term_input(&self, target_node: &Url) -> AppendTermInput {
        let mut logs_guard = self.logs.lock().await;
        // prev term is the latest term the remote node should have
        let prev_term = self.get_target_term(target_node, &mut logs_guard).await;
        let (created, local_latest_term) = logs_guard.latest();
        if created {
            self.hook.append_term(&local_latest_term);
        }
        let leader_id = self.settings.node_id.clone();
        let leader_commit_index = logs_guard.commit_index();

        // Add up to 10 entries only
        let mut pos = prev_term.id + 1;
        let end = pos + 10;

        // Case 1:
        // The latest term the remote has is also my term.
        if prev_term == local_latest_term {
            debug!("just send latest because previous term IS local latest");
            return AppendTermInput {
                term: local_latest_term.clone(),
                leader_id,
                prev_term: local_latest_term,
                entries: vec![],
                leader_commit_index,
            };
        } else if pos >= local_latest_term.id {
            debug!("just send latest because previous term is just before our local latest");
            return AppendTermInput {
                term: local_latest_term.clone(),
                leader_id,
                prev_term: local_latest_term,
                entries: vec![],
                leader_commit_index,
            };
        }

        // Case 2:
        // The latest term the remote is older than our.
        // :=> prev_term.id + 1 <= local_latest_term.id
        //     <=> pos <= local_latest_term.id
        let entries = {
            let mut retreived = vec![];
            while pos < end && pos < local_latest_term.id - 1 {
                if let Some(term) = logs_guard.find(pos) {
                    retreived.push(term);
                } else if let Some(term) = self.hook.retreive_term(pos) {
                    retreived.push(term);
                } else {
                    panic!("impossible to retrieve a term as a leader");
                }
                pos += 1
            }
            retreived
        };

        // Limit the knowledge of distant node to the latest
        // term. Note: I'm not so sure about that.
        let term = if pos == local_latest_term.id {
            local_latest_term
        } else if let Some(term) = logs_guard.find(pos) {
            term
        } else if let Some(term) = self.hook.retreive_term(pos) {
            term
        } else {
            panic!("impossible to retrieve a term as a leader");
        };

        AppendTermInput {
            term,
            leader_id,
            prev_term,
            entries,
            leader_commit_index,
        }
    }
}
