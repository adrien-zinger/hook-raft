// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::Node;
use tracing::{debug, trace};

impl Node {
    pub(crate) async fn commit_entries(
        &self,
        old_commit_index: usize,
        new_commit_index: usize,
    ) {
        let logs = self.logs.lock().await;
        for entry in logs.iter_range(old_commit_index..new_commit_index + 1) {
            debug!("commit entry {}", entry.id);
            trace!("commit entry {} {}", entry.id, entry.content);
            // todo: make that a constant (the "conn")
            if let Some(u) = entry.parse_conn() {
                if u.hash == self.uuid {
                    trace!(
                        "local node has been accepted by the current leader"
                    );
                    continue; // I'm ok with me
                }
                trace!("add a new node in the local index {}", u.addr);
                if u.follower {
                    self.follower_list.write().await.insert(u.addr);
                    trace!("follower list incremented");
                } else {
                    self.node_list.write().await.insert(u.addr);
                    trace!("node list incremented");
                }
            }
            self.hook.commit_term(&entry);
        }
    }
}
