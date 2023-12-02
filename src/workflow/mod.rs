// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

//! Module declaration for all workflows described in initial specifications
//! Look at dev documentation for more information

pub mod append_term;
pub mod candidate;
pub mod follower;
pub mod init;
pub mod leader;
pub mod request_vote;
mod tools;

#[cfg(test)]
mod test;
