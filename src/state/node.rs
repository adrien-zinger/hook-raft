// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.
use super::EStatus;
use crate::{common::error::ErrorResult, state::Url, Node};

impl Node {
    pub(crate) async fn switch_to_candidate(&self) -> ErrorResult<()> {
        self.p_status.switch_to_candidate().await?;
        self.hook.switch_status(EStatus::Candidate);
        Ok(())
    }

    pub(crate) async fn switch_to_leader(&self) -> ErrorResult<()> {
        self.p_status.switch_to_leader().await?;
        self.hook.switch_status(EStatus::Leader);
        Ok(())
    }

    pub(crate) async fn switch_to_follower(&self, leader: Url) -> ErrorResult<()> {
        if !self.p_status.is_follower().await {
            self.p_status.switch_to_follower(leader).await?;
            self.hook.switch_status(EStatus::Follower);
        }
        Ok(())
    }
}
