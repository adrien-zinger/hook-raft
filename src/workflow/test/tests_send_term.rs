// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::{io_msg::UpdateNodeInput, Url},
    common::{config::Settings, scripts::DefaultHook},
    log_entry::Entries,
    node::generate_uuid,
    state::{Leader, Status},
    workflow::leader::_term_preparation,
    Node,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use tokio::sync::{Mutex, RwLock};
use tracing_test::traced_test;

#[tokio::test]
#[serial_test::serial]
#[traced_test]
async fn term_preparation_1() {
    // Check if nodes_waiting is poped correctly
    let mut nodes_waiting = VecDeque::new();
    let hash = generate_uuid();
    nodes_waiting.push_front(
        serde_json::to_string(&UpdateNodeInput {
            hash,
            port: "8080".to_string(),
            follower: true,
        })
        .unwrap(),
    );
    let node = Node {
        waiting_nodes: Arc::new(Mutex::new(nodes_waiting)),
        ..Node::_init(
            Settings::default(),
            Status::<Leader>::create(),
            DefaultHook {},
        )
    };
    for _ in 0..100 {
        _term_preparation(
            &node.logs,
            &node.p_status,
            &node.waiting_nodes,
            &node.node_list,
            &node.hook,
        )
        .await;
        if node.waiting_nodes.lock().await.is_empty() {
            break;
        }
    }
    let nodes_waiting = node.waiting_nodes.lock().await;
    let logs = node.logs.lock().await;
    assert!(nodes_waiting.is_empty());
    assert!(!logs.len() > 0);
    assert!(logs.back().is_some());
    assert!(logs.back().unwrap().content.starts_with("conn:"));

    let mut c = logs.back().unwrap().content;
    c.drain(.."conn:".len());
    let conn_to: UpdateNodeInput = serde_json::from_str(&c.to_string()).unwrap();
    assert_eq!(conn_to.hash, hash);
    assert!(conn_to.follower);
}

#[tokio::test]
#[serial_test::serial]
#[traced_test]
async fn create_term_for_node_1() {
    let known_follower = Url::from("127.0.0.1:8081");

    // first time we speak with him, we don't know where he is in his logs

    let node = Node {
        leader: Arc::new(RwLock::new(Some(Url::from("127.0.0.1:8080")))),
        logs: Arc::new(Mutex::new(Entries::default())),
        ..Node::_init(
            Settings::default(),
            Status::<Leader>::create(),
            DefaultHook {},
        )
    };
    let input = node.create_term_input(&known_follower).await;
    assert_eq!(input.entries.len(), 0);
    assert_eq!(input.prev_term.id, 0);
    assert_eq!(input.term.id, 1);
    assert_eq!(input.term.content, "");

    let input = node.create_term_input(&known_follower).await;
    assert_eq!(input.entries.len(), 0);
    assert_eq!(input.prev_term.id, 1);
    assert_eq!(input.term.id, 2);
    assert_eq!(input.term.content, "");

    logs_assert(|logs| {
        let mut c = 0;
        for log in logs.iter() {
            if log.contains("create an empty term") {
                c += 1;
            }
        }
        if c == 2 {
            Ok(())
        } else {
            // we should just once trigger a warning,
            // because the next call use the term created
            // from the previous call
            Err("invalid number of warnings".to_string())
        }
    });
}

#[tokio::test]
#[serial_test::serial]
#[traced_test]
async fn create_term_for_node_2() {
    let known_follower = Url::from("127.0.0.1:8081");

    // first time we speak with him, we don't know where he is in his logs

    // initialize some logs
    let mut entries = Entries::new();
    entries.append("1st term".to_string());
    entries.append("2nd term".to_string());
    entries.append("3rd term".to_string());
    entries.append("4th term".to_string());

    let node = Node {
        leader: Arc::new(RwLock::new(Some(Url::from("127.0.0.1:8080")))),
        logs: Arc::new(Mutex::new(entries)),
        ..Node::_init(
            Settings::default(),
            Status::<Leader>::create(),
            DefaultHook {},
        )
    };
    let input = node.create_term_input(&known_follower).await;
    assert_eq!(input.entries.len(), 0);
    assert_eq!(input.prev_term.id, 3);
    assert_eq!(input.prev_term.content, "4th term");
    assert_eq!(input.term.id, 3);
}

#[tokio::test]
#[serial_test::serial]
#[traced_test]
async fn create_term_for_node_3() {
    // same as create_term_for_node_2 but target already know some logs
    let known_follower = Url::from("127.0.0.1:8081");

    let mut next_indexes = HashMap::default();
    // target know 3 entries in logs
    next_indexes.insert(known_follower.clone(), 2);

    // initialize some logs
    let mut entries = Entries::new();
    entries.append("1st term".to_string());
    entries.append("2nd term".to_string());
    entries.append("3rd term".to_string());
    entries.append("4th term".to_string());

    let node = Node {
        follower_list: Arc::new(RwLock::new(follower_list)),
        next_indexes: Arc::new(RwLock::new(next_indexes)),
        leader: Arc::new(RwLock::new(Some(Url::from("127.0.0.1:8080")))),
        logs: Arc::new(Mutex::new(entries)),
        ..Node::_init(
            Settings::default(),
            Status::<Leader>::create(),
            DefaultHook {},
        )
    };
    let input = node.create_term_input(&known_follower).await;
    // should be empty because there is just one new log, in "term"
    assert_eq!(input.entries.len(), 0);
    // id of the 4th term
    assert_eq!(input.term.id, 3);
    // content of the 4th term
    assert_eq!(input.term.content, "4th term");
    // previous term is the next_indexe of the target
    assert_eq!(input.prev_term.id, 2)
}

#[tokio::test]
#[serial_test::serial]
#[traced_test]
async fn create_term_for_node_4() {
    // same as create_term_for_node_3 but target already know other logs
    let known_follower = Url::from("127.0.0.1:8081");

    let mut next_indexes = HashMap::default();
    // target know 2 entries in logs
    next_indexes.insert(known_follower.clone(), 1);

    // initialize some logs
    let mut entries = Entries::new();
    entries.append("1st term".to_string());
    entries.append("2nd term".to_string());
    entries.append("3rd term".to_string());
    entries.append("4th term".to_string());

    let node = Node {
        next_indexes: Arc::new(RwLock::new(next_indexes)),
        leader: Arc::new(RwLock::new(Some(Url::from("127.0.0.1:8080")))),
        logs: Arc::new(Mutex::new(entries)),
        ..Node::_init(
            Settings::default(),
            Status::<Leader>::create(),
            DefaultHook {},
        )
    };
    let input = node.create_term_input(&known_follower).await;
    // should be empty because there is just one new log, in "term"
    assert_eq!(input.entries.len(), 1);
    // the entry in the list should be the 3rd
    assert!(input.entries.find(2).is_some());
    // id of the 4th term
    assert_eq!(input.term.id, 3);
    // content of the 4th term
    assert_eq!(input.term.content, "4th term");
    // previous term is the next_indexe of the target
    assert_eq!(input.prev_term.id, 1)
}

#[tokio::test]
#[serial_test::serial]
#[traced_test]
async fn create_term_for_node_5() {
    let known_follower = Url::from("127.0.0.1:8081");

    let mut next_indexes = HashMap::default();
    // target know everything
    next_indexes.insert(known_follower.clone(), 3);

    // initialize some logs
    let mut entries = Entries::new();
    entries.append("1st term".to_string());
    entries.append("2nd term".to_string());
    entries.append("3rd term".to_string());
    entries.append("4th term".to_string());

    let node = Node {
        follower_list: Arc::new(RwLock::new(follower_list)),
        next_indexes: Arc::new(RwLock::new(next_indexes)),
        leader: Arc::new(RwLock::new(Some(Url::from("127.0.0.1:8080")))),
        logs: Arc::new(Mutex::new(entries)),
        p_commit_index: Arc::new(Mutex::new(2)),
        ..Node::_init(
            Settings::default(),
            Status::<Leader>::create(),
            DefaultHook {},
        )
    };

    let input = node.create_term_input(&known_follower).await;
    assert_eq!(input.entries.len(), 0);
    // id of the 4th term
    assert_eq!(input.term.id, 3);
    // content of the 4th term
    assert_eq!(input.term.content, "4th term");
    // previous term is the next_indexe of the target
    assert_eq!(input.prev_term.id, 3);

    assert_eq!(input.leader_commit_index, 2);
}
