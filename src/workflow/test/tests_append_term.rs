// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::io_msg::AppendTermInput,
    common::{config::Settings, scripts::DefaultHook},
    log_entry::{Entries, Term},
    node::Node,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_test::traced_test;

#[tokio::test]
#[serial_test::serial]
#[traced_test]
async fn tests_append_term_as_pending_node() {
    let node = Node::_init(
        Settings::default(),
        Status::<ConnectionPending>::create(),
        DefaultHook {},
    );
    // Leader does not know the node we're testing,
    // he send just the latest term twice, (in term and prev_term)
    // the leader commit index is 11
    node.receive_append_term(AppendTermInput {
        term: Term::_new(12, "12th term"),
        leader_id: "127.0.0.12:1212".to_string(),
        prev_term: Term::_new(12, "12th term"),
        entries: Entries::new(),
        leader_commit_index: 11,
    })
    .await
    .unwrap();
    // now the node is a follower
    assert!(node.p_status._is_follower().await);
    // latest commited is 11
    assert_eq!(*node.p_commit_index.lock().await, 11);
    // logs has term 0, and 12
    assert_eq!(node.logs.lock().await.len(), 2);
    // what we just commited
    logs_assert(|a| {
        let mut count = 0;
        for ele in a.iter().clone() {
            println!("{ele}");
            if ele.contains("commit") {
                count += 1;
            }
        }
        if count != 1 {
            return Err("We should commit just the 0".to_string());
        }
        Ok(())
    });
}

#[tokio::test]
#[serial_test::serial]
async fn tests_append_term_too_forward() {
    let mut node = Node::_init(
        Settings::default(),
        Status::<Follower>::create(),
        DefaultHook {},
    );
    let response = node
        .receive_append_term(AppendTermInput {
            term: Term::_new(12, "12th term"),
            leader_id: "127.0.0.12:1212".to_string(),
            prev_term: Term::_new(12, "11th term"),
            entries: Entries::new(),
            leader_commit_index: 11,
        })
        .await
        .unwrap();

    assert_eq!(response.current_term.id, 0);
    assert!(!response.success);

    let mut entries = Entries::new();
    entries.append("1st term".to_string());
    entries.append("2nd term".to_string());
    entries.append("3rd term".to_string());
    entries.append("4th term".to_string());
    entries.append("5th term".to_string());
    entries.append("6th term".to_string());
    node.logs = Arc::new(Mutex::new(entries));
    *node.p_current_term.lock().await = Term::_new(5, "6th term");
    let response = node
        .receive_append_term(AppendTermInput {
            term: Term::_new(12, "12th term"),
            leader_id: "127.0.0.12:1212".to_string(),
            prev_term: Term::_new(12, "11th term"),
            entries: Entries::new(),
            leader_commit_index: 11,
        })
        .await
        .unwrap();
    assert_eq!(response.current_term.id, 5);
    assert!(!response.success);
}

#[tokio::test]
#[serial_test::serial(receive)]
async fn tests_append_term_with_entries() {
    let node = Node::_init(
        Settings::default(),
        Status::<Follower>::create(),
        DefaultHook {},
    );
    let mut entries = Entries::new();
    entries.append("1st term".to_string());
    entries.append("2nd term".to_string());
    entries.append("3rd term".to_string());
    entries.append("4th term".to_string());
    entries.append("5th term".to_string());
    entries.append("6th term".to_string());
    entries.append("7th term".to_string());
    entries.append("8th term".to_string());
    entries.append("9th term".to_string());
    entries.append("10th term".to_string());
    entries.append("11th term".to_string());
    let response = node
        .receive_append_term(AppendTermInput {
            term: Term::_new(12, "12th term"),
            leader_id: "127.0.0.12:1212".to_string(),
            prev_term: Term::_new(0, ""),
            entries,
            leader_commit_index: 4,
        })
        .await
        .unwrap();
    assert_eq!(response.current_term.id, 12);
    assert!(response.success);
    assert_eq!(node.p_current_term.lock().await.id, 12);
    assert_eq!(node.p_current_term.lock().await.content, "12th term");
    assert_eq!(*node.p_commit_index.lock().await, 4);
}

// todo: test if a leader become a follower with a new term
// todo: test usecases where a check fail
// todo: test usecases where a append_term work
// todo: create a new test file with a leader that run and send terms
//       - make some scenarios
