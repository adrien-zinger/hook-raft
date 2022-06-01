// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::{client, io_msg::UpdateNodeResult, server},
    common::error::{errors, throw, Error, ErrorResult},
    node::{Node, NodeInfo},
    state::EStatus,
};
use tracing::trace;

impl Node {
    /// First workflow described in `init.md` specification. The folowing workflow
    /// is called only once time when the node start.
    ///
    /// - Start a server in a tokio task (then run in background).
    /// - Manage the case where there is no known nodes in the user settings.
    /// - Try to connect to each nodes taken from the `nodes` variable in the
    ///   `settings.toml`.
    ///
    /// If success, it should know the leader address (`leader_id`). And try to
    /// connect to it.
    ///
    /// # Error
    /// The error should be managed by the root caller of the function and cause
    /// at the end a graceful stop of the node.
    pub async fn initialize(&self) -> ErrorResult<()> {
        {
            let e_status = self.p_status.read().await;
            match &*e_status {
                EStatus::ConnectionPending(status) => status.clone(),
                _ => throw!(Error::WrongStatus),
            }
        };
        let node_clone = self.clone();
        tokio::spawn(async move { server::new(node_clone).await });
        if self.settings.nodes.is_empty() {
            eprintln!("warn: No nodes known, may be a configuration error");
            if self.settings.follower {
                throw!(errors::ERR_FOLLOWER_MUST_HAVE_KNOWN)
            } else {
                self.set_status_leader().await?;
                return Ok(());
            }
        }
        self.connect_to_leader().await?;
        Ok(())
    }

    /// Workflow function on receive a connection request. Connection request are
    /// done by a `update_node` call. Take a `UpdateNodeInput` conatining the
    /// calling node and if he want to be a follower.
    ///
    /// If you are a leader, you add the follower into a pool that is managed in
    /// the workflow `On heartbeat timeout` in `leader_workflow`.
    ///
    /// Whatever your status, if the script `update-node` succed it returns
    /// an `UpdateNodeResult` and none otherwise.
    pub async fn receive_connection_request(
        &self,
        input: NodeInfo,
    ) -> Option<UpdateNodeResult> {
        trace!("receive connection request from {}", input.addr);
        if !self.hook.update_node() {
            return None;
        }
        if let EStatus::Leader(_) = &*self.p_status.read().await {
            self.node_list.write().await.remove(&input.addr);
            self.follower_list.write().await.remove(&input.addr);
            self.push_new_waiting_node(input).await;
            return Some(UpdateNodeResult {
                leader_id: format!(
                    "{}:{}",
                    self.settings.addr, self.settings.port
                ),
                node_list: self
                    .node_list
                    .read()
                    .await
                    .iter()
                    .cloned()
                    .collect(),
                follower_list: self
                    .follower_list
                    .read()
                    .await
                    .iter()
                    .cloned()
                    .collect(),
            });
        };
        match &*self.leader.read().await {
            Some(leader_id) => Some(UpdateNodeResult {
                leader_id: leader_id.to_string(),
                node_list: self
                    .node_list
                    .read()
                    .await
                    .iter()
                    .cloned()
                    .collect(),
                follower_list: self
                    .follower_list
                    .read()
                    .await
                    .iter()
                    .cloned()
                    .collect(),
            }),
            None => {
                eprintln!(
                    "cannot create an `UpdateNodeResult` without a leader"
                );
                None
            }
        }
    }

    async fn push_new_waiting_node(&self, node_info: NodeInfo) {
        trace!("leader event: push a new waiting node {}", node_info.addr);
        let ser = serde_json::to_string(&node_info).unwrap();
        let mut waiting_nodes = self.waiting_nodes.lock().await;
        if waiting_nodes.contains(&ser) {
            eprintln!(
                "warn: url `{}` already in the connection pool",
                &node_info.addr
            );
        } else {
            waiting_nodes.push_back(ser);
        }
    }

    /// Called in `initialize` for the connection to a leader. Try to connect to each known
    /// `nodes` from the settings.toml
    ///
    /// # Error
    /// The error should be managed by the root caller of the function and cause
    /// at the end a graceful stop of the node.
    async fn connect_to_leader(&self) -> ErrorResult<()> {
        let mut success = false;
        let mut to_leader = false;
        for url in self.settings.nodes.iter() {
            match client::post_update_node(
                &url.into(),
                &self.settings,
                self.uuid,
            )
            .await
            {
                Ok(result) => {
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
            throw!(Error::ImpossibleToBootstrap);
        }
        trace!("connection {} to leader {}", success, to_leader);
        if !to_leader {
            let leader = self.leader.read().await.clone().unwrap();
            match client::post_update_node(
                &leader.clone(),
                &self.settings,
                self.uuid,
            )
            .await
            {
                Ok(result) => self.update(result).await,
                Err(warn) => {
                    throw!(Error::CannotStartRpcServer(format!(
                        "Failed to connect to the leader: `{}`\n{:indent$?}",
                        leader,
                        *warn,
                        indent = 2
                    )))
                }
            }
        }
        trace!("connection success");
        Ok(())
    }

    async fn update(&self, result: UpdateNodeResult) {
        trace!("update leader {}", result.leader_id);
        self.node_list.write().await.extend(result.node_list);
        self.follower_list
            .write()
            .await
            .extend(result.follower_list);
        *self.leader.write().await = Some(result.leader_id.into());
    }
}
