// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    common::{
        config::{self, Settings},
        error::{throw, Error, ErrorResult},
        hook_trait::Hook,
        Url,
    },
    log_entry::Entries,
    state::{EStatus, Status},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use tokio::{
    runtime::Runtime,
    sync::{oneshot::Sender, Mutex, RwLock},
    task::JoinHandle,
};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    filter::filter_fn, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
};

pub enum NextIndex {
    Validated(usize),
    Pending(usize),
}

impl NextIndex {
    pub fn unwrap(&self) -> usize {
        match self {
            NextIndex::Validated(val) => *val,
            NextIndex::Pending(val) => *val,
        }
    }

    pub fn validated(&self) -> usize {
        match self {
            NextIndex::Validated(val) => *val,
            NextIndex::Pending(_) => 1, /* return minimum */
        }
    }
}

#[derive(Clone)]
pub struct Node {
    /// Current state of the local node
    pub p_status: Status,
    /// Settings of the node
    pub settings: Settings,
    /// Heartbeat dynamic timeout used by follower (who can be candidates)
    pub heartbeat: Arc<Mutex<Option<Sender<()>>>>,
    /// Log entries, terms are stored here until they are committed
    pub logs: Arc<Mutex<Entries>>,
    /// Next indexes by know nodes (table of None at initialization)
    /// Is initialized on comes to power.
    pub next_indexes: Arc<RwLock<HashMap<Url, NextIndex>>>,
    /// Wait to connect
    pub waiting_nodes: Arc<Mutex<VecDeque<String>>>,
    /// List of nodes that can be potential leader and candidates
    /// Note only these nodes votes
    pub node_list: Arc<RwLock<HashSet<String>>>,
    /// Last vote Some(node id, last log term) if voted, None otherwise
    pub vote_for: Arc<RwLock<Option<(String, usize)>>>,
    /// hook interface
    pub hook: Arc<Box<dyn Hook>>,
    /// Unique node id, used as a temporary identifier in the network
    pub uuid: [u8; 16],
    /// Container for mock return values in some unit tests
    #[cfg(test)]
    pub utest_data: UTestData,
}

/// Container for mock return values in some unit tests
#[cfg(test)]
#[derive(Clone, Default)]
pub struct UTestData {
    pub error_result_bool: Option<ErrorResult<bool>>,
}

// todo: verify if leader correctly update the `last_applied` and call the
//       `apply_term` script each time he create a term
// todo: the logs (Entries structure) should prune the oldest committed terms
//       with a kind of buffer (size in the settings)
// todo: add a maximum for logs production
// todo: we need to define what should be in the debug level of tracing.

impl Node {
    /// Private default implementation
    fn default(settings: Settings, hook: impl Hook + 'static) -> Self {
        Self {
            p_status: Status::connection_pending(),
            heartbeat: Default::default(),
            logs: Default::default(),
            next_indexes: Default::default(),
            waiting_nodes: Default::default(),
            node_list: Arc::new(RwLock::new(HashSet::from_iter(
                settings.nodes.iter().cloned(),
            ))),
            settings,
            vote_for: Default::default(),
            hook: Arc::new(Box::new(hook)),
            uuid: generate_uuid(),
            #[cfg(test)]
            utest_data: Default::default(),
        }
    }

    /// Creates a new default node
    pub fn new(hook: impl Hook + 'static) -> Self {
        let layer = tracing_subscriber::fmt::layer()
            .with_filter(filter_fn(|metadata| metadata.target().starts_with("hook")))
            .with_filter(LevelFilter::TRACE); // todo: use an input or a setting for log level
        tracing_subscriber::registry()
            // add the console layer to the subscriber or default layers...
            .with(layer)
            .init();

        let opt_path = if std::env::args().len() > 1 {
            Some(std::env::args().collect::<Vec<String>>()[1].clone())
        } else {
            None
        };
        let settings = match config::read(opt_path) {
            Ok(settings) => settings,
            Err(err) => {
                eprintln!(
                    "Cannot read configuration file, getting default settings\n{:?}",
                    err
                );
                Settings::default()
            }
        };
        Self::default(settings, hook)
    }

    #[cfg(test)]
    /// Creates a node with the given settings and status
    pub fn test_new(settings: Settings, p_status: Status, hook: impl Hook + 'static) -> Self {
        Self {
            p_status,
            ..Self::default(settings, hook)
        }
    }

    pub fn new_with_settings(settings: Settings, hook: impl Hook + 'static) -> Self {
        Self {
            ..Self::default(settings, hook)
        }
    }

    async fn internal_main_loop(&self) -> ErrorResult<()> {
        self.initialize().await?;
        loop {
            if self.p_status.is_pending().await {
                self.p_status.wait();
            }
            let status_loop = async {
                match self.p_status.status().await {
                    EStatus::Leader => self.run_leader().await,
                    EStatus::Follower => self.run_follower().await,
                    EStatus::Candidate => self.run_candidate().await,
                    EStatus::ConnectionPending => {
                        throw!(Error::WrongStatus)
                    }
                }
            };
            tokio::pin!(status_loop);
            tokio::select! {
                res = status_loop => {
                    if let Err(err) = res {
                        throw!(*err)
                    }
                },
                _ = tokio::signal::ctrl_c() => {
                    println!("Handle a graceful shutdown");
                    break
                },
            }
        }
        Ok(())
    }

    /// Start a node inside a given tokio `runtime`
    pub fn start(&self, runtime: Runtime) -> ErrorResult<()> {
        runtime.block_on(async { self.internal_main_loop().await })
    }

    /// Spawn new loop
    pub fn spawn(self) -> JoinHandle<ErrorResult<()>> {
        tokio::spawn(async move { self.internal_main_loop().await })
    }

    pub(crate) async fn get_node_list(&self) -> Vec<String> {
        self.node_list.read().await.iter().cloned().collect()
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NodeInfo {
    pub hash: [u8; 16],
    pub addr: String,
}

pub(crate) fn generate_uuid() -> [u8; 16] {
    let mut ret = [0u8; 16];
    for a in ret.iter_mut() {
        *a = rand::random();
    }
    ret
}
