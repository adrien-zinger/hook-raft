#![allow(unused)]
use crate::{api::io_msg::UpdateNodeResult, common::config::Settings};
use crate::{
    api::Url,
    common::error::{WarnResult, Warning},
};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    static ref POST_UPDATE_NODE_RES: Arc<Mutex<VecDeque<WarnResult<UpdateNodeResult>>>> = Default::default();
}

pub async fn post_update_node(_url: &Url, _settings: &Settings) -> WarnResult<UpdateNodeResult> {
    match POST_UPDATE_NODE_RES.lock().await.pop_front() {
        Some(ret) => ret,
        None => panic!("Mock should be fill by results before use"),
    }
}

pub async fn fill_post_update_node_res(queue: &mut VecDeque<WarnResult<UpdateNodeResult>>) {
    let mut q = POST_UPDATE_NODE_RES.lock().await;
    q.clear();
    q.append(queue);
}
