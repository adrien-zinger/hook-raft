// License:
// This source code is licensed under the GPLv3 license, you can found the
// LICENSE file in the root directory of this source tree.

use crate::{
    api::{client::fill_post_update_node_res, io_msg::UpdateNodeResult},
    common::{
        config::Settings,
        error::{Error, Warning},
    },
    node::Node,
    state::{ConnectionPending, Status},
};
use std::collections::VecDeque;

#[tokio::test]
#[serial_test::serial]
async fn test_initialize_success() {
    // The following should success because the response leader_id will be one in the list
    let leader_id = "10.10.10.10:1212".to_string();
    let node_list = vec!["10.10.10.10:1212".to_string()];
    let follower_list = vec!["12.12.12.12:1212".to_string()];
    let mut res = VecDeque::default();
    res.push_back(Ok(UpdateNodeResult {
        leader_id,
        node_list: node_list.clone(),
        follower_list,
    }));
    let settings = Settings {
        nodes: node_list,
        follower: false,
        ..Default::default()
    };
    fill_post_update_node_res(&mut res).await;
    let p_status = Status::<ConnectionPending>::create();
    let mut node = Node {
        // default init
        ..Node::_init(settings, p_status)
    };
    node.initialize().await.unwrap();
}

#[tokio::test]
#[serial_test::serial]
async fn test_initialize_err_command_fail() {
    // The following should success because the response leader_id will be one in the list
    let node_list = vec!["10.10.10.10:1212".to_string()];
    let mut res = VecDeque::default();
    res.push_back(Err(Box::new(Warning::CommandFail(
        "command fail".to_string(),
    ))));
    let settings = Settings {
        nodes: node_list,
        ..Default::default()
    };
    fill_post_update_node_res(&mut res).await;
    let p_status = Status::<ConnectionPending>::create();
    let mut node = Node {
        // default init
        ..Node::_init(settings, p_status)
    };
    match node.initialize().await {
        Ok(_) => panic!("Unexpected connection success"),
        Err(err) => {
            if !matches!(*err, Error::ImpossibleToBootstrap) {
                panic!("Unexpected error {:?}", err)
            }
        }
    };
}

#[tokio::test]
#[serial_test::serial]
async fn test_initialize_fail_leader_timeout() {
    // The following should success because the response leader_id will be one in the list
    let leader_id = "10.10.10.10:1212".to_string();
    let node_list = vec!["11.11.11.11:1212".to_string()];
    let follower_list = vec!["12.12.12.12:1212".to_string()];
    let mut res = VecDeque::default();

    // Connection to the first node 11.11.11.11 succeded
    res.push_front(Ok(UpdateNodeResult {
        leader_id,
        node_list: node_list.clone(),
        follower_list,
    }));
    // Connection to the leader fail
    res.push_front(Err(Box::new(Warning::Timeout("Connect timeout"))));

    let settings = Settings {
        nodes: node_list,
        ..Default::default()
    };
    fill_post_update_node_res(&mut res).await;
    let p_status = Status::<ConnectionPending>::create();
    let mut node = Node {
        // default init
        ..Node::_init(settings, p_status)
    };
    match node.initialize().await {
        Ok(_) => panic!("Unexpected connection success"),
        Err(p_err) => {
            if matches!(*p_err, Error::CannotStartRpcServer(_)) {
                panic!("Unexpected error {:?}", *p_err)
            }
        }
    };
}

#[tokio::test]
#[tokio::serial]
async fn test_follower_without_node_fail() {
    let settings = Settings {
        follower: true, // default but set for information =-)
        ..Default::default()
    };
    let p_status = Status::<ConnectionPending>::create();
    let mut node = Node {
        // default init
        ..Node::_init(settings, p_status)
    };
    node.initialize().await.expect_err("Expected error here");
}
