use crate::{
    api::Url,
    common::{config::Settings, error::Warning, scripts::DefaultHook},
    node::{generate_uuid, Node},
    state::{ConnectionPending, Leader, Status},
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[tokio::test]
#[serial_test::serial]
async fn update_node() {
    let settings = Settings::default(); // server settings
    let node = Node {
        ..Node::_init(
            settings.clone(),
            Status::<ConnectionPending>::create(),
            DefaultHook {},
        )
    };
    let sig_stop = Arc::new(AtomicBool::new(true));
    let sig_stop_c = sig_stop.clone();
    // run server
    let jh = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.spawn(async {
            let sp = tokio::spawn(async move {
                super::server::new(node).await.unwrap();
            });
            sp.await.unwrap();
        });
        while sig_stop_c.load(Ordering::Relaxed) {}
        rt.shutdown_background();
    });

    std::thread::sleep(Duration::from_millis(200));
    let target = format!("{}:{}", settings.addr, settings.port);
    match super::client::post_update_node(
        &target.into(),
        &settings, /* No need to recreate another settings here. */
        generate_uuid(),
    )
    .await
    {
        Err(err) => match *err {
            Warning::BadResult(err) => println!("Warning:\n{}", err),
            _ => panic!("Unexpected result"),
        },
        _ => panic!("Unexpected result"),
    };
    sig_stop.store(false, Ordering::Relaxed);
    jh.join().unwrap();
}

#[tokio::test]
#[serial_test::serial]
async fn update_node_and_call_directly_the_leader() {
    let settings = Settings::default(); // server settings
    let node = Node {
        leader: Url::get_ptr("127.0.0.1:3001"),
        ..Node::_init(settings.clone(), Status::<Leader>::create(), DefaultHook {})
    };
    let sig_stop = Arc::new(AtomicBool::new(true));
    let sig_stop_c = sig_stop.clone();
    // run server
    let jh = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.spawn(async {
            let sp = tokio::spawn(async move {
                super::server::new(node).await.unwrap();
            });
            sp.await.unwrap();
        });
        while sig_stop_c.load(Ordering::Relaxed) {}
        rt.shutdown_background();
    });

    std::thread::sleep(Duration::from_millis(200));
    let target = format!("{}:{}", settings.addr, settings.port);
    match super::client::post_update_node(
        &target.into(),
        &settings, /* No need to recreate another settings here. */
        generate_uuid(),
    )
    .await
    {
        Ok(res) => println!("result:\n{:?}", res),
        _ => panic!("Unexpected result"),
    };
    sig_stop.store(false, Ordering::Relaxed);
    jh.join().unwrap();
}
