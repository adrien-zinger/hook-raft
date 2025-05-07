use crate::common::error::{throw, Error};
use config::Config;
use rand::Rng;
use serde::Deserialize;
use tokio::time::Duration;

use super::error::ErrorResult;

/***********************************************/
/*  DEFAULT CONFIGURATION                      */

fn default_addr() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> String {
    "3000".to_string()
}
const fn default_follower() -> bool {
    false
}
const fn default_response_timeout() -> usize {
    20
}
const fn default_nodes() -> Vec<String> {
    vec![]
}
const fn default_timeout_min() -> usize {
    150
}
const fn default_timeout_max() -> usize {
    300
}
const fn default_prepare_term_period() -> u64 {
    80
}
const fn default_node_id() -> String {
    String::new()
}

/// Represent the user settings in the settings.toml
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_timeout_min")]
    pub timeout_min: usize,
    #[serde(default = "default_timeout_max")]
    pub timeout_max: usize,
    #[serde(default = "default_nodes")]
    pub nodes: Vec<String>,
    #[serde(default = "default_addr")]
    pub addr: String,
    #[serde(default = "default_port")]
    pub port: String,
    #[serde(default = "default_follower")]
    pub follower: bool,
    #[serde(default = "default_response_timeout")]
    pub response_timeout: usize,
    #[serde(default = "default_prepare_term_period")]
    pub prepare_term_period: u64,
    #[serde(default = "default_node_id")]
    pub node_id: String,
}

impl Settings {
    /// Compute a random heartbeat timeout before it start a candidate
    /// workflow. Use range `[timeout_min..=timeout_max]`
    pub fn get_randomized_timeout(&self) -> Duration {
        let mut rng = rand::thread_rng();
        Duration::from_millis(rng.gen_range(self.timeout_min..=self.timeout_max) as u64)
    }
    pub fn get_prepare_term_sleep_duration(&self) -> Duration {
        Duration::from_millis(self.prepare_term_period)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            timeout_min: default_timeout_min(),
            timeout_max: default_timeout_max(),
            nodes: default_nodes(),
            addr: default_addr(),
            port: default_port(),
            follower: default_follower(),
            response_timeout: default_response_timeout(),
            prepare_term_period: default_prepare_term_period(),
            node_id: default_node_id(),
        }
    }
}

/// Read the config file `settings.toml` and parse the node configuration.
///
/// Done when we are creating (de-referencing) the `StatusPtr` in `node.rs` for
/// the first time.
pub fn read(opt_path: Option<String>) -> ErrorResult<Settings> {
    let path = opt_path.unwrap_or_else(|| "settings.toml".to_string());
    let config = Config::builder()
        .add_source(config::File::with_name(&path))
        .build()
        .unwrap(); // todo remove the unwrap, prefer a nice error
    match config.try_deserialize::<Settings>() {
        Ok(settings) => Ok(settings),
        Err(error) => throw!(Error::CannotReadSettings(std::sync::Arc::new(error))),
    }
    // todo check if configuration is OK, (example: timeout_min < timeout_max)
}
