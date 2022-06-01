#[cfg(not(feature = "mock_api"))]
pub mod client;
pub mod mock;
#[cfg(feature = "mock_api")]
pub mod client {
    pub use super::mock::mock_client::*;
}

pub mod io_msg;
pub mod server;

mod url;
pub use url::*;

#[cfg(test)]
#[cfg(not(feature = "mock_api"))]
mod tests;
