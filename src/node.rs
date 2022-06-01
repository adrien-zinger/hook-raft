// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::Url,
    common::{
        config::{self, Settings},
        error::{throw, Error, ErrorResult},
        hook_trait::Hook,
    },
    log_entry::{Entries, Term},
    state::{ConnectionPending, EStatus, EmptyStatus, Status, StatusPtr},
};
use dyn_timeout::tokio_impl::DynTimeout;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};
use tokio::{
    runtime::Runtime,
    sync::{Mutex, RwLock},
    task::JoinHandle,
};
use tracing::{metadata::LevelFilter, trace};
use tracing_subscriber::{
    filter::filter_fn, prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt, Layer,
};

#[derive(Clone)]
pub struct Node {
    /// Current state of the local node
    pub p_status: StatusPtr,
    /// Settings of the node
    pub settings: Settings,
    /// Heartbeat dynamic timeout used by follower (who can be candidates)
    pub opt_heartbeat: Arc<Mutex<Option<DynTimeout>>>,
    /// Log entries, each entry contains command for state machine, and term
    /// when entry was received by Leader.
    pub logs: Arc<Mutex<Entries>>,
    /// latest term server has seen (initialized to 0 on first boot, increases
    /// monotonically), usually it's the latest log in self.logs
    pub p_current_term: Arc<Mutex<Term>>,
    /// Stored commited index (usually min(leader.commited_index,
    /// logs.max_index))
    pub p_commit_index: Arc<Mutex<usize>>,
    /// Next indexes by know nodes (table of None at initialization)
    /// Is initialized on comes to power.
    pub next_indexes: Arc<RwLock<HashMap<Url, usize>>>,
    // todo: node waiting could be a simple vector of tuple
    /// Wait to connect
    pub waiting_nodes: Arc<Mutex<VecDeque<String>>>,
    /// List of nodes that can be potential leader and candidates
    /// Note only these nodes votes
    pub node_list: Arc<RwLock<HashSet<String>>>,
    /// List of nodes that are only followers
    pub follower_list: Arc<RwLock<HashSet<String>>>,
    /// Leader id that is also his IP
    pub leader: Arc<RwLock<Option<Url>>>,
    /// Last vote Some(node id, last log term) if voted, None otherwise
    pub vote_for: Arc<RwLock<Option<(String, Term)>>>,
    /// hook interface
    pub hook: Arc<Box<dyn Hook>>,
    /// Unique node id, used as a temporary identifier in the network
    pub uuid: [u8; 16],
}

// todo: define a nodelist pointer structure and implement, insert, to_vec,
//       contains...
// todo: make the ctrl-c very sensitive (you'll debug a lot if you start that
//       quest)
// todo: verify if leader correctly update the `last_applied` and call the
//       `apply_term` script each time he create a term
// todo: the logs (Entries structure) should prune the oldest commited terms
//       with a kind of buffer (size in the settings)
// todo: add a maximum for logs production
// todo: we need to define what should be in the debug level of tracing.

impl Node {
    /// Private default implementation
    fn default(settings: Settings, hook: impl Hook + 'static) -> Self {
        Self {
            p_status: Status::<ConnectionPending>::create(),
            opt_heartbeat: Default::default(),
            logs: Default::default(),
            p_current_term: Default::default(),
            p_commit_index: Default::default(),
            next_indexes: Default::default(),
            waiting_nodes: Default::default(),
            node_list: Arc::new(RwLock::new(HashSet::from_iter(
                settings.nodes.iter().cloned(),
            ))),
            settings,
            follower_list: Default::default(),
            leader: Default::default(),
            vote_for: Default::default(),
            hook: Arc::new(Box::new(hook)),
            uuid: generate_uuid(),
        }
    }

    /// Creates a new default node
    pub fn new(hook: impl Hook + 'static) -> Self {
        let layer = tracing_subscriber::fmt::layer()
            .with_filter(filter_fn(|metadata| {
                metadata.target().starts_with("hook")
            }))
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
    pub fn _init(
        settings: Settings,
        p_status: StatusPtr,
        hook: impl Hook + 'static,
    ) -> Self {
        Self {
            p_status,
            ..Self::default(settings, hook)
        }
    }

    pub fn new_with_settings(
        settings: Settings,
        hook: impl Hook + 'static,
    ) -> Self {
        Self {
            ..Self::default(settings, hook)
        }
    }

    async fn internal_main_loop(&self) -> ErrorResult<()> {
        self.initialize().await?;
        loop {
            let st = {
                match &*self.p_status.read().await {
                    EStatus::Follower(_) => EmptyStatus::Follower,
                    EStatus::Candidate(_) => EmptyStatus::Candidate,
                    EStatus::Leader(_) => EmptyStatus::Leader,
                    EStatus::ConnectionPending(_) => {
                        EmptyStatus::ConnectionPending
                    }
                }
            };
            if matches!(st.clone(), EmptyStatus::ConnectionPending) {
                trace!("waiting to connect");
                let sleep = tokio::time::sleep(Duration::from_millis(500));
                tokio::pin!(sleep);
                tokio::select! {
                    _ = sleep => {
                        continue
                    },
                    _ = tokio::signal::ctrl_c() => {
                        println!("Handle a gracefull shotdown");
                        break
                    },
                }
            }
            let status_loop = async {
                match st {
                    EmptyStatus::Leader => self.run_leader().await,
                    EmptyStatus::Follower => self.run_follower().await,
                    EmptyStatus::Candidate => self.run_candidate().await,
                    EmptyStatus::ConnectionPending => {
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
                    println!("Handle a gracefull shotdown");
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
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NodeInfo {
    pub hash: [u8; 16],
    pub addr: String,
    pub follower: bool,
}

pub(crate) fn generate_uuid() -> [u8; 16] {
    let mut ret = [0u8; 16];
    for a in ret.iter_mut() {
        *a = rand::random();
    }
    ret
}
