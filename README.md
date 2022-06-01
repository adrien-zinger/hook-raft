# Hook / Raft Library

Hook is the _Proof Of Concept_ of an implementation of the raft algorithm. It want to divide the work of a node from the raft logic (inspiration from hooks in git).

The implementation follow the rules of the Raft algorithm during his
execution expect for the connection of new nodes. The connection
process is described in [the init specification](./specs/initial flow charts/init.md).

## _Principia raftica_

The raft algorithm works with terms, and that's what hook does. By default the terms are empty because it doesn't have to be important in a first place. Even if you'll be able to hack this project and fills terms, the main target of hook is to know who's the leader in the network.

So Hook will connect all nodes of the network (accepting new nodes with a bootstrap strategy) and send basic pings as terms for the raft.

## Hook-Raft? 🪝

That's all? Noooo, I called the project Hook because it implements a logic as the hooks in git repository.

There is a trait named *Hook* that you can implement however you want. That trait is given to the hook library through the entry point that is:

```rust
/// Start a new node
fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Runtime expected to start but failed");
    match Node::new(DefaultHook {}).start(rt) {
        Ok(_) => println!("Successfully exit"),
        Err(err) => eprintln!("Node crash with error: {:?}", err),
    }
}
```

You can also define the settings manually with the function
`new_with_settings`, otherwise the library will look at a file named
`settings.toml` in the root folder. Look below what are the settings.


The Trait `Hook` can be a default **VOID** with the `DefaulHook`
object but can be whatever you want. This object is basically an observer
that the *Raft* algorithm will trigger on change.

Let's look at the disponible hooks trigerred:

```bash
└── hooks
    ├── pre-append-term
    ├── append-term
    ├── apply-term # todo
    ├── commit-term
    ├── update-node
    ├── leader-change # todo
    ├── pre-add-connection # todo
    ├── post-add-connection # todo
    ├── remove-connection # todo
    ├── lost-connection # todo
    ├── request-vote # todo
    ├── receive-vote # todo
    ├── prepare-term
    └── send-term # todo
```

-   _pre-append-term_: A term append from the leader, the result of the script has to be "true", otherwise the term is rejected (Hook accepts all term by default)
-   _apply-term_: You have a term with a content and you want to apply something to a state machine or something? It's here
-   _append-term_: A term as just passed all the test and is stored in cache, waiting to be commited. The same term can be append several times
-   _commit-term_: The term is considered as definitive by the current leader
-   _leader-change_: A new leader won the election
-   _pre-add-connection_: A new node want to join the network, the return of the script has to be "true" you accept the connection. All other response is considered has rejected. (Look at [default hook requirement](#default-hook-node-requirements))
-   _post-add-connection_: A connection has been added in the network
-   _remove-connection_: A node has been removed from the network
-   _lost-connecton_: A node as failed to answer, or the leader failed to send heartbeat
-   _request-vote_: Vote requested for a new leader
-   _receive-vote_: Received a response for your personal vote request if you are a candidate
-   _prepare-term_: If you are the leader, you can fill the content with the of `terms` with this script, the returned value is considered as the full raw content. This script is called `PREPARING_OFFSET` ms before each heartbeat.
-   _send-term_: It's call when you're a leader, and you send a term, you can check if you successfully prepare the last term here
-   _update-node_: A node require to connect or to change his status (follower/not follower) You can dismiss every connection if you want or limit the number of nodes

The status of the implementation is watched with the `config` crates and is updated on save in an acceptable time.

### Raft settings

When starting a new node, you can target a settings file. Note that you shouldn't run a node on a network with random settings because you may fail to connect.

```toml
timeout_min = 150
timeout_max = 300
# Min and max value in milisecond of the election timeout. Timeout is randomly choosen between these two values

max_timeout_value = 300
# Maximum value in millisecond of the heartbeat timeout if you're a potential candidate
min_inc_timeout = 150
min_inc_timeout = 300
# Min and max value in millisecond of the random incrementation of ou timeout each time we received a new term.

prepare_term_period = 80
# Value in milisecond that separe term preparations, default 80
# If this time is too short to finish the term preparation, an empty heartbeat
# will be send and the content will be used for the next term. The hook doesn't
# implement any problem management if you fail multiple times to send a term.
# You can manage it yourself with the `send-term` script

nodes = ['12.158.20.36']
# Optional list of public known nodes in a network. If this list appear to be empty, the node won't connect to anything and will be the current leader of his own network.

# todo followers = ['15.126.208.72']
# Optional list of known followers

addr = "127.0.0.1"
# Server local address, default "127.0.0.1"
port = "3000"
# Port used between nodes to communicate, default "3000"

follower = true
# If true, the current node will never ask for an election and will never be able to vote. Nevertheless you will receive all heartbeat and all information like a normal node. Some hooks will never be called obviously but you are a part of the network. If false, you will be considered as a potential candidate after a successfull bootstrap and will be able to vote.
# default true

response_timeout = 20
# Value in millisecond before considering that a node will never respond
```

## Run The node!

_todo: can use hook raft as a library, or use a ready to use executable that is in a subproject._

## Default Hook Node Requirements

Hook accepts all connection by default if requirements are ok.

Rejected causes:

-   The node want to be candidate but failed to communicate with another randomly chosen node in the follower list (I don't know if it's really useful)
-   One of the nodes rejected your connection

## Bootstrapping

The bootstrap system isn't managed here. If you want to implement a bootstrap strategy, you can develop it your own server who use hook and check in the `update-node` script if the node who attempt to connect fill requirements.

However, _Hook-Raft_ implementation is bootstrap friendly and will send by default all logs commited from the latest log known by the connecting node. That part is, after all, suceptible to move soon with the nexts _PR_! The ideas tempt to extract that process of logs retreivals from here. So in the future, the connecting node will certainly receive only the latest log commited.


