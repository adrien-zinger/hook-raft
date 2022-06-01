// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

//!
//! ```ignore
//! /// Start a new node
//! fn main() {
//!     let rt = tokio::runtime::Runtime::new().expect("Runtime expected to start but failed");
//!     match Node::new(DefaultHook {}).start(rt) {
//!         Ok(_) => println!("Successfully exit"),
//!         Err(err) => eprintln!("Node crash with error: {:?}", err),
//!     }
//! }
//! ```

mod api;
mod common;
mod log_entry;
mod node;
mod state;
mod workflow;

pub use common::config::Settings;
pub use common::hook_trait::Hook;
pub use log_entry::Term;
pub use node::Node;
