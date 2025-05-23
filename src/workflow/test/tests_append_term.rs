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

#[cfg(test)]
fn get_simple_follower(leader_url: String) -> Node {
    let settings = Settings {
        follower: false,
        nodes: vec![leader_url.clone()],
        ..Default::default()
    };
    Node {
        p_status: Status::follower(leader_url.into()),
        ..Node::new_with_settings(
            settings,
            TestHook {
                pre_append_terms: Arc::new(StdMutex::new(vec![1].into())),
                ..TestHook::default()
            },
        )
    }
}

#[tokio::test]
async fn tests_append_term() {
    let leader_url = String::from("10.10.10.10:1212");
    let node = get_simple_follower(leader_url.clone());
    let _ = node
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

    // The node is a follower so it should have accepted the term
    // and current term id is 1.
    assert_eq!(curr_term.id, 1)
}

#[tokio::test]
async fn case_one_false() {
    /* ****
    Test the first point of the official spec:
        1. Reply false if term < currentTerm (§5.1)
    **** */
    let leader_url = String::from("10.10.10.10:1212");
    let node = get_simple_follower(leader_url.clone());

    // Setup the node with some terms. Response should be ok.
    let res1 = node
        .receive_append_term(AppendTermInput {
            term: Term::_new(3, "3rd term"),
            leader_id: leader_url.clone(),
            prev_term: Term::_new(1, "1st term"),
            entries: vec![Term::_new(2, "2nd term")],
            leader_commit_index: 0,
        })
        .await
        .unwrap();

    //assert_eq!(res1.current_term, 3);
    assert!(res1.success);
}

#[tokio::test]
async fn case_one_false_setup_failure() {
    /* ****
    The `case_one_test` is initialized with some Terms, just
    to tell that the initialization must be complete (all entries)
    has to be sent. Make a scenario with a gap between term 1 an 3.

    Note that it's also related with the second point of the spec:
        2. Reply false if log doesn’t contain an entry at prevLogIndex
           whose term matches prevLogTerm (§5.3)
    **** */

    let leader_url = String::from("10.10.10.10:1212");
    let node = get_simple_follower(leader_url.clone());

    let res1 = node
        .receive_append_term(AppendTermInput {
            term: Term::_new(3, "3rd term"),
            leader_id: leader_url.clone(),
            prev_term: Term::_new(1, "1st term"),
            entries: vec![/* missing Term 2 */],
            leader_commit_index: 0,
        })
        .await
        .unwrap();

    assert!(!res1.success)
}
