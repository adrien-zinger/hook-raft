// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::io_msg::UpdateNodeResult,
    common::error::{throw, Error, ErrorResult},
    node::{Node, NodeInfo},
};
use tracing::{trace, warn};

impl Node {
    /// First workflow described in `init.md` specification. The following workflow
    /// is called only once time when the node start.
    ///
    /// - Start a server in a tokio task (then run in background).
    /// - Manage the case where there is no known nodes in the user settings.
    /// - Try to connect to each nodes taken from the `nodes` variable in the
    ///   `settings.toml`.
    ///
    /// If success, it should know the leader address (`leader_id`). Then try to
    /// connect to it.
    ///
    /// # Error
    /// The error should be managed by the root caller of the function and cause
    /// at the end a graceful stop of the node.
    pub async fn initialize(&self) -> ErrorResult<()> {
        if !self.p_status.is_pending().await {
            throw!(Error::WrongStatus)
        }
        let node_clone = self.clone();
        #[cfg(not(test))] // no server in unit test
        tokio::spawn(async move { crate::api::server::new(node_clone).await });
        if self.settings.nodes.is_empty() {
            eprintln!("warn: No nodes known, may be a configuration error");
        }
        if !self.connect_to_leader().await? {
            // No connection possible, turn into a follower.
            if !self.settings.follower {
                self.switch_to_candidate().await?;
            }
        }
        Ok(())
    }

    /// Workflow function on receive a connection request. Connection request are
    /// done by a `update_node` call. Take a `UpdateNodeInput` containing the
    /// calling node and if he want to be a follower.
    ///
    /// If you are a leader, you add the follower into a pool that is managed in
    /// the workflow `On heartbeat timeout` in `leader_workflow`.
    ///
    /// Whatever your status, if the script `update-node` succeed it returns
    /// an `UpdateNodeResult` and none otherwise.
    pub async fn receive_connection_request(&self, input: NodeInfo) -> Option<UpdateNodeResult> {
        trace!("receive connection request from {}", input.addr);
        if !self.hook.update_node() {
            return None;
        }
        if self.p_status.is_leader().await {
            // self.node_list.write().await.remove(&input.addr);
            return Some(UpdateNodeResult {
                leader_id: format!("{}:{}", self.settings.addr, self.settings.port),
                node_list: self.get_node_list().await,
            });
        };

        let leader_id = if let Some(leader_id) = self.p_status.leader().await {
            leader_id.to_string()
        } else {
            String::new()
        };
        Some(UpdateNodeResult {
            leader_id,
            node_list: self.get_node_list().await,
        })
    }

    /// Called in `initialize` for the connection to a leader. Try to connect to each known
    /// `nodes` from the settings.toml
    ///
    /// # Error
    /// The error should be managed by the root caller of the function and cause
    /// at the end a graceful stop of the node.
    #[cfg(not(test))] // mocked in unit tests
    async fn connect_to_leader(&self) -> ErrorResult<bool> /* TODO just return bool? */ {
        use crate::api::client;

        let mut success = false;
        let mut to_leader = false;
        for url in self.settings.nodes.iter() {
            match client::post_update_node(&url.into(), &self.settings, self.uuid).await {
                Ok(result) => {
                    /* Succeed to send an update node request */
                    success = true;
                    to_leader = result.leader_id == *url;
                    self.update(result).await;
                    break;
                }
                Err(warn) => eprintln!(
                    "Connection warning, distant node: `{url}`, {:indent$?}",
                    warn,
                    indent = 2
                ),
            };
        }
        if !success {
            /* No connection possible */
            return Ok(false);
        }
        trace!("connection {} to leader {}", success, to_leader);
        if !to_leader {
            /*
             * Connected to the network, but not to the leader.
             *
             * On bootstrap, if the leader is unknown, we just wait
             * for an event. A new candidate can show up, or we can
             * become a leader on a timeout.
             *
             * If we know who's the leader, we want to signal that we
             * exist and we're actually running. */
            let leader = match self.p_status.leader().await {
                Some(leader) => leader,
                _ => return Ok(false),
            };
            match client::post_update_node(&leader.clone(), &self.settings, self.uuid).await {
                Ok(result) => self.update(result).await,
                Err(warn) => {
                    warn!(
                        "Failed to connect to the leader: `{}`\n{:indent$?}",
                        leader,
                        *warn,
                        indent = 2
                    );
                    return Ok(false);
                }
            }
        }
        trace!("connection success");
        Ok(true)
    }

    async fn update(&self, result: UpdateNodeResult) {
        trace!("update leader {}", result.leader_id);
        // self.node_list.write().await.extend(result.node_list);
        self.p_status
            .switch_to_follower(result.leader_id.into())
            .await
            .unwrap();
    }
}
