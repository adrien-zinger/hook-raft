use hook_raft::*;

/// Start a new node
fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Runtime expected to start but failed");
    match Node::new(DefaultHook {}).start(rt) {
        Ok(_) => println!("Successfully exit"),
        Err(err) => eprintln!("Node crash with error: {:?}", err),
    }
}

