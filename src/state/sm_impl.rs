// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use super::{
    Candidate, ConnectionPending, EStatus, Follower, Leader, Status, StatusPtr,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::trace;

// todo: we can try to create our own traits that allow us to give some
//       arguments and check if the state flow is forbidden or not.
//       (ex: assert!(!settings.follower, "A follower cannot be a leader");)

impl From<Status<ConnectionPending>> for Status<Leader> {
    fn from(_: Status<ConnectionPending>) -> Self {
        // todo: assert!(!settings.follower, "A follower cannot be a leader");
        trace!("become a leader");
        Status::<Leader>::default()
    }
}

impl From<Status<ConnectionPending>> for Status<Follower> {
    fn from(_: Status<ConnectionPending>) -> Self {
        trace!("become a follower");
        Status::<Follower>::default()
    }
}

impl From<Status<Follower>> for Status<Candidate> {
    fn from(_: Status<Follower>) -> Self {
        trace!("become a candidate");
        Status::<Candidate>::default()
    }
}

impl From<Status<Candidate>> for Status<Follower> {
    fn from(_: Status<Candidate>) -> Self {
        trace!("become a follower");
        Status::<Follower>::default()
    }
}

impl From<Status<Candidate>> for Status<Leader> {
    fn from(_: Status<Candidate>) -> Self {
        trace!("become a leader");
        Status::<Leader>::default()
    }
}

impl From<Status<Leader>> for Status<Follower> {
    fn from(_: Status<Leader>) -> Self {
        trace!("become a follower");
        Status::<Follower>::default()
    }
}

impl Status<ConnectionPending> {
    /// Create a StatusPtr at the initial and unique status that we
    /// can create: `ConnectionPending`.
    pub fn create() -> StatusPtr {
        trace!("initialize status to ConnectionPending");
        StatusPtr(Arc::new(RwLock::new(EStatus::ConnectionPending(Status::<
            ConnectionPending,
        > {
            t: Default::default(),
        }))))
    }
}

impl Status<Leader> {
    #[cfg(test)]
    pub fn create() -> StatusPtr {
        StatusPtr(Arc::new(RwLock::new(EStatus::Leader(Status::<Leader> {
            t: Default::default(),
        }))))
    }
}

impl Status<Follower> {
    #[cfg(test)]
    pub fn create() -> StatusPtr {
        StatusPtr(Arc::new(RwLock::new(EStatus::Follower(
            Status::<Follower> {
                t: Default::default(),
            },
        ))))
    }
}

// Contain the number of nodes
impl<T> Status<T> {
    fn default() -> Self {
        Status::<T> {
            t: Default::default(),
        }
    }
}
