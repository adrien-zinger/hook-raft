// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

//! Implementation of useful [Node] methods for changing his state.

use super::EStatus;
use crate::{
    common::error::{throw, Error, ErrorResult},
    Node,
};
use tracing::trace;

impl Node {
    /// Set the `node` status to candidate, return an error if he can't.
    pub(crate) async fn set_status_to_candidate(&self) -> ErrorResult<()> {
        let mut e_status = self.p_status.write().await;
        let status = match &*e_status {
            EStatus::Follower(status) => status.clone(),
            _ => throw!(Error::WrongStatus),
        };
        *e_status = EStatus::Candidate(status.into());
        Ok(())
    }

    /// Set the `node` status to leader, return an error if he can't.
    pub(crate) async fn set_status_leader(&self) -> ErrorResult<()> {
        let mut e_status = self.p_status.write().await;
        let status = match &*e_status {
            EStatus::Candidate(status) => status.clone().into(),
            EStatus::ConnectionPending(status) => status.clone().into(),
            _ => throw!(Error::WrongStatus),
        };
        *e_status = EStatus::Leader(status);
        Ok(())
    }

    ///     Candidate --> Follower
    ///     Leader --> Follower
    ///     Follower --> Follower
    pub(crate) async fn set_status_to_follower(
        &self,
        leader: String,
    ) -> ErrorResult<()> {
        trace!("set status to follower");
        let mut guard = self.p_status.write().await;
        *self.leader.write().await = Some(leader.into());
        let status = match &*guard {
            EStatus::Follower(_) => return Ok(()),
            EStatus::Candidate(status) => status.clone().into(),
            EStatus::Leader(status) => status.clone().into(),
            EStatus::ConnectionPending(status) => status.clone().into(),
        };
        *guard = EStatus::Follower(status);
        Ok(())
    }
}
