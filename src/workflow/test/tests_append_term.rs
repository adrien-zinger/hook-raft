// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::io_msg::AppendTermInput,
    common::{config::Settings, scripts::DefaultHook},
    log_entry::{Entries, Term},
    node::Node,
    state::Status,
    workflow::test::hook::TestHook,
};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use tracing_test::traced_test;
/*
internal implementation of receive append term

Specification
1. Reply false if term < currentTerm (§5.1)
2. Reply false if log doesn’t contain an entry at prevLogIndex
whose term matches prevLogTerm (§5.3)
3. If an existing entry conflicts with a new one (same index
but different terms), delete the existing entry and all that
follow it (§5.3)
4. Append any new entries not already in the log
5. If leaderCommit > commitIndex, set commitIndex =
min(leaderCommit, index of last new entry
*/

#[tokio::test]
async fn tests_append_term() {
    let leader_url = String::from("10.10.10.10:1212");

    let settings = Settings {
        follower: false,
        nodes: vec![leader_url.clone()],
        ..Default::default()
    };
    let node = Node {
        p_status: Status::follower(leader_url.clone().into()),
        ..Node::new_with_settings(
            settings,
            TestHook {
                pre_append_terms: Arc::new(StdMutex::new(vec![1].into())),
                ..TestHook::default()
            },
        )
    };

    let res = node
        .receive_append_term(AppendTermInput {
            term: Term::_new(1, "1st term"),
            leader_id: leader_url,
            prev_term: Term::_new(1, "1st term"),
            entries: vec![],
            leader_commit_index: 0,
        })
        .await
        .unwrap();

    let curr_term: Term = node.logs.lock().await.current_term();
    assert_eq!(curr_term.id, 1)
}
