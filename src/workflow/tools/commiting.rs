// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::Node;
use tracing::{debug, trace, warn};

impl Node {
    /// Commit up to the new commit index included.
    /// Saturate and return if the index to commit isn't in cache.
    pub(crate) async fn commit_entries(&self, new_commit_index: usize) {
        let mut logs = self.logs.lock().await;
        let from = logs.commit_index() + 1;
        if from <= new_commit_index {
            trace!("commit entries term from {from} to {new_commit_index}");
            for index in from..=new_commit_index {
                if !logs.check_commit(index) {
                    debug!("stop commit at log term {index}");
                    break;
                }
                let term = match logs.find(index) {
                    Some(term) => term,
                    None => {
                        warn!("unable to find a log while committing");
                        break;
                    }
                };
                debug!("commit term {:?}", term);
                logs.set_commit(index);
                self.hook.commit_term(&term);
            }
        }
    }
}
