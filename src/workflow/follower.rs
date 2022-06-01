// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

//! Implementation of the folower workflow, a follower start a heartbeat
//! timeout if node's settings say it's not a pure follower (can be candidate)
//! If the node is a follower follower, doesn't start any timeout.

use crate::{common::error::ErrorResult, node::Node};
use dyn_timeout::tokio_impl::DynTimeout;
use tokio::sync::mpsc;
use tracing::trace;

impl Node {
    /// Start follower workflow
    pub async fn run_follower(&self) -> ErrorResult<()> {
        trace!("start follower workflow");
        if self.settings.follower {
            trace!("run until ctrl-c");
            tokio::select! { _ = tokio::signal::ctrl_c() => {} };
            Ok(())
        } else {
            let (sender, mut receiver) = mpsc::channel::<()>(1);
            let dur = self.settings.get_randomized_timeout();
            let mut dyn_timeout = DynTimeout::with_sender(dur, sender);
            dyn_timeout
                .set_max_waiting_time(self.settings.get_max_timeout_value());
            trace!("start timeout");
            *self.opt_heartbeat.lock().await = Some(dyn_timeout);
            tokio::select! {
                _ = receiver.recv() => {
                    trace!("heartbeat timeout!");
                    *self.opt_heartbeat.lock().await = None;
                    self.set_status_to_candidate().await?
                }
                _ = tokio::signal::ctrl_c() => {}
            }
            Ok(())
        }
    }
}
