// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

/// Initialization require mock api. Launch tests with `cargo test --features mock_api`
mod tests_append_term;
#[cfg(feature = "mock_api")]
mod tests_init;
mod tests_send_term;
