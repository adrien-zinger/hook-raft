// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

//! Implementation of the follower workflow, a follower start a heartbeat
//! timeout if node's settings say it's not a pure follower (can be candidate)
//! If the node is a follower follower, doesn't start any timeout.

use crate::{common::error::ErrorResult, node::Node};
use tracing::{debug, trace};

impl Node {
    /// Start follower workflow
    pub async fn run_follower(&self) -> ErrorResult<()> {
        trace!("start follower workflow");
        if self.settings.follower {
            trace!("run until ctrl-c");
            tokio::select! { _ = tokio::signal::ctrl_c() => {} };
            Ok(())
        } else {
            self.reset_timeout().await;
            while self.p_status.is_follower().await {
                self.p_status.wait();
            }
            Ok(())
        }
    }

    pub async fn reset_timeout(&self) {
        let p_heartbeat = self.heartbeat.clone();
        let p_status = self.p_status.clone();
        let (send, mut recv) = tokio::sync::oneshot::channel::<()>();
        let dur = self.settings.get_randomized_timeout();

        let mut heartbeat = self.heartbeat.lock().await;
        if heartbeat.is_some() {
            // cancel previous heartbeat timeout by triggering
            // the `recv` branch in the select bellow
            heartbeat.take();
        }
        *heartbeat = Some(send);
        tokio::spawn(async move {
            debug!("start new timeout");
            let sleep = tokio::time::sleep(dur);
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut recv => debug!("cancel previous timeout"),
                _ = &mut sleep => {
                    debug!("branch heartbeat timeout reached");
                    p_heartbeat.lock().await.take();
                    let _ = p_status.switch_to_candidate().await;
                }
            }
        });
    }
}
