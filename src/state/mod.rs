// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

//! Description of the "states" of the node. In Raft, a node can be:
//! - A leader
//! - A Follower
//! - A candidate
//!
//! In some cases, the connection is just pending. Whatever, there is a
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

use std::{marker::PhantomData, sync::Arc};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
pub mod node;
pub mod sm_impl;

#[derive(Clone)]
pub struct ConnectionPending;
#[derive(Clone)]
pub struct Follower;
#[derive(Clone)]
pub struct Candidate;
#[derive(Clone)]
pub struct Leader;

/// Enum used by the Node to contain his current state.
/// Look at StatusPtr that Arc the Enum.
pub enum EStatus {
    /// The node is trying to connect to others
    ConnectionPending(Status<ConnectionPending>),
    /// The node is a follower inthe network
    /// It can be a candidate in a while if the configuration
    /// allow it.
    Follower(Status<Follower>),
    /// The node is a candidate in his network
    Candidate(Status<Candidate>),
    /// The node is the leader in his network
    Leader(Status<Leader>),
}

/// Same as the [EStatus] but used in case we just want to conserve the last
/// state without interact with the internal status.
#[derive(Clone)]
pub enum EmptyStatus {
    Leader,
    Follower,
    Candidate,
    ConnectionPending,
}

/***********************************************/

#[derive(Clone)]
pub struct StatusPtr(Arc<RwLock<EStatus>>);

// todo: Upgrade that implementation, allow user to give an opional guard,
//       change the names of the functions.
impl StatusPtr {
    /// Check if leader
    pub async fn _is_leader(&self) -> bool {
        let guard = self.0.read().await;
        matches!(*guard, EStatus::Leader(_))
    }
    /// Check if connection pending
    pub async fn _is_pending(&self) -> bool {
        let guard = self.0.read().await;
        matches!(*guard, EStatus::ConnectionPending(_))
    }
    /// Check if connection is follower
    pub async fn _is_follower(&self) -> bool {
        let guard = self.0.read().await;
        matches!(*guard, EStatus::Follower(_))
    }
    pub async fn _is_candidate(&self) -> bool {
        let guard = self.0.read().await;
        matches!(*guard, EStatus::Candidate(_))
    }
    pub async fn read(&'_ self) -> RwLockReadGuard<'_, EStatus> {
        self.0.read().await
    }
    pub async fn write(&'_ self) -> RwLockWriteGuard<'_, EStatus> {
        self.0.write().await
    }
}

#[derive(Clone)]
pub struct Status<T> {
    t: PhantomData<T>,
}
