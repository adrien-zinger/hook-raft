// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

//! Description of the "states" of the node. In Raft, a node can be:
//! - A leader
//! - A Follower
//! - A candidate
//!
//! In some cases, the connection is just pending. However, there is a
//! specific workflow to follow to be in a given state.
//!
//! ```mermaid
//! flowchart
//! ConnectionPending --> |on accepted by the leader\nlook at the leader_workflow| Follower
//! ConnectionPending --> |"if follower == false && nodes is empty"| Leader
//! Follower --> |if follower == false| Candidate
//! Candidate --> Leader
//! Candidate --> Follower
//! Leader --> Follower
//! ```
//!
//! We control the state flow in that module, allowing only the creation of a
//! [Status<ConnectionPending>] and force the modification through the
//! implementation of some `From` traits. Implementations are in [sm_impl].

use crate::common::{
    error::{throw, Error, ErrorResult},
    Url,
};
use std::sync::{Arc, Condvar, Mutex};
use tokio::sync::RwLock;
use tracing::trace;

mod node;

#[derive(Clone, Copy)]
pub enum EStatus {
    ConnectionPending,
    Follower,
    Candidate,
    Leader,
}

/***********************************************/
/* "public" access to the status */
/***********************************************/

#[derive(Clone)]
pub struct Status {
    inner: Arc<RwLock<(EStatus, Option<Url>)>>,
    cv: Arc<(Mutex<()>, Condvar)>,
}

#[cfg(test)]
impl Status {
    /// Test feature. Create a leader status.
    pub fn leader() -> Status {
        let inner = Arc::new(RwLock::new((EStatus::Leader, None)));
        let cv = Arc::new((Mutex::new(()), Condvar::new()));
        Status { inner, cv }
    }

    /// Test feature. Create a follower status.
    pub fn follower(leader: Url) -> Status {
        let inner = Arc::new(RwLock::new((EStatus::Follower, Some(leader))));
        let cv = Arc::new((Mutex::new(()), Condvar::new()));
        Status { inner, cv }
    }

    /// Test feature. Create a follower status.
    pub fn candidate() -> Status {
        let inner = Arc::new(RwLock::new((EStatus::Candidate, None)));
        let cv = Arc::new((Mutex::new(()), Condvar::new()));
        Status { inner, cv }
    }
}

impl Status {
    /// Wait a modification of the status
    pub(crate) fn wait(&self) {
        let (mutex, condvar) = &*self.cv;
        let guard = mutex.lock().unwrap();
        let _unused = condvar.wait(guard).unwrap();
    }

    pub(crate) async fn status(&self) -> EStatus {
        self.inner.read().await.0
    }

    /// Switch the current status to candidate.
    /// Follower -> Candidate
    pub(crate) async fn switch_to_candidate(&self) -> ErrorResult<()> {
        let mut inner = self.inner.write().await;
        match inner.0 {
            EStatus::Follower => { /* OK */ }
            EStatus::ConnectionPending => { /* OK */ }
            _ => throw!(Error::WrongStatus),
        }
        trace!("switch to candidate");
        *inner = (EStatus::Candidate, None);
        let (mutex, condvar) = &*self.cv;
        let _guard = mutex.lock().unwrap();
        condvar.notify_all();
        Ok(())
    }

    /// Switch the current status to leader.
    /// Candidate -> Leader
    pub(crate) async fn switch_to_leader(&self) -> ErrorResult<()> {
        let mut inner = self.inner.write().await;
        match inner.0 {
            EStatus::Candidate => { /* OK */ }
            _ => throw!(Error::WrongStatus),
        }
        trace!("switch to leader");
        *inner = (EStatus::Leader, None);
        let (mutex, condvar) = &*self.cv;
        let _guard = mutex.lock().unwrap();
        condvar.notify_all();
        Ok(())
    }

    /// Switch the current status to follower.
    /// Every state can turn into a follower
    pub(crate) async fn switch_to_follower(&self, leader: Url) -> ErrorResult<()> {
        trace!("switch to follower");
        let mut inner = self.inner.write().await;
        *inner = (EStatus::Follower, Some(leader));
        let (mutex, condvar) = &*self.cv;
        let _guard = mutex.lock().unwrap();
        condvar.notify_all();
        Ok(())
    }

    /// Create a connection pending status, which is the default status.
    pub fn connection_pending() -> Status {
        let inner = Arc::new(RwLock::new((EStatus::ConnectionPending, None)));
        let cv = Arc::new((Mutex::new(()), Condvar::new()));
        Status { inner, cv }
    }

    pub async fn get_leader(&self) -> Option<Url> {
        self.inner.read().await.1.clone()
    }

    pub async fn is_pending(&self) -> bool {
        matches!(&self.inner.read().await.0, EStatus::ConnectionPending)
    }

    pub async fn is_candidate(&self) -> bool {
        matches!(&self.inner.read().await.0, EStatus::Candidate)
    }

    pub async fn is_leader(&self) -> bool {
        matches!(&self.inner.read().await.0, EStatus::Leader)
    }

    pub async fn is_follower(&self) -> bool {
        matches!(&self.inner.read().await.0, EStatus::Follower)
    }
}
