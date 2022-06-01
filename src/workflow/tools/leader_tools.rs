// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::{io_msg::AppendTermInput, Url},
    log_entry::{Entries, Term},
    node::Node,
};
use tokio::sync::MutexGuard;
use tracing::warn;

impl Node {
    async fn get_prev_term<'a>(
        &self,
        target_node: &Url,
        logs_guard: &mut MutexGuard<'a, Entries>,
    ) -> Term {
        match self.next_indexes.read().await.get(target_node) {
            Some(node_index) => {
                let id = *node_index;
                match logs_guard.find(id) {
                    Some(term) => term,
                    _ => logs_guard.back().unwrap(),
                }
            }
            _ => match logs_guard.back() {
                Some(term) => term,
                None => {
                    warn!("create an empty term in get previous context");
                    logs_guard.append(String::new());
                    Term {
                        id: logs_guard.len() - 1,
                        content: String::new(),
                    }
                }
            },
        }
    }

    async fn build_current_term<'a>(
        &self,
        logs_guard: &mut MutexGuard<'a, Entries>,
    ) -> Term {
        let ret = {
            let o = match logs_guard.back() {
                Some(term) => {
                    if term == *self.p_current_term.lock().await {
                        None
                    } else {
                        Some(term)
                    }
                }
                _ => None,
            };
            match o {
                Some(term) => term,
                None => {
                    warn!("create an empty term in get current context");
                    logs_guard.append(String::new());
                    Term {
                        id: logs_guard.len() - 1,
                        content: String::new(),
                    }
                }
            }
        };
        ret
    }

    /// Creates a term especially for the `target_node`
    pub(crate) async fn create_term_input(
        &self,
        target_node: &Url,
    ) -> AppendTermInput {
        let mut logs_guard = self.logs.lock().await;
        let prev_term = self.get_prev_term(target_node, &mut logs_guard).await;
        let term = self.build_current_term(&mut logs_guard).await;
        let leader_id =
            format!("{}:{}", self.settings.addr, self.settings.port);
        let entries = if prev_term.id + 1 < term.id {
            match logs_guard.get_copy_range(prev_term.id + 1..term.id) {
                Some(e) => e,
                _ => Entries::new(),
            }
        } else {
            Entries::new()
        };
        *self.p_current_term.lock().await = term.clone();
        let leader_commit_index = *self.p_commit_index.lock().await;
        AppendTermInput {
            term,
            leader_id,
            prev_term,
            entries,
            leader_commit_index,
        }
    }
}
