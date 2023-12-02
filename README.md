# Hook / Raft Library

Hook is an implementation of the raft algorithm. It wants to extracts the
algorithm, just doing the election and manage the logs transfers between
nodes.

The implementation follow the rules of the Raft algorithm during his
execution.

## _Principia raftica_

The raft algorithm works with terms, and that's what hook does.
The hook's goal is to commit a consensual approved a maximum of terms.
By default, terms are empty because it doesn't have to be important in a first
place. But you can choose what's inside a term by using a *hook*.

## Hook-Raft? 🪝

Hook implements a logic as the hooks in a git repository. You're able
to implement some behaviors that interact with the basic Raft algorithm.

First, it allows any user to define a content of terms. But it can also
interact with the default behaviors. That's let anyone to hack/try some
configuration easily.

There is a trait named *Hook*. That trait is given to the hook library through
the entry point that is:

```rust
pub trait Hook: Send + Sync {
    fn update_node(&self) -> bool;
    fn pre_append_term(&self, term: &Term) -> Option<usize>;
    fn append_term(&self, term: &Term) -> bool;
    fn commit_term(&self, term: &Term) -> bool;
    fn prepare_term(&self) -> String;
    fn retreive_term(&self, index: usize) -> Option<Term>;
    fn retreive_terms(&self, from: usize, to: usize) -> Option<Vec<Term>>;
    fn switch_status(&self, status: EStatus);
}
```

You can also define the settings manually with the function
`new_with_settings`, otherwise the library will look at a file named
`settings.toml` in the root folder. Look below what are the settings.

The Trait `Hook` can be a default **VOID** with the `DefaulHook`
object but can be whatever you want. This object is basically an observer
that the *Raft* algorithm will trigger any time it require.

## Default Hook

The default hook binary will react with the following scripts or executable.
All of that script are optional, put a '.sample' extension or remove it to
enable the internal default behavior.

```bash
└── hook
    ├── hook.bin
    ├── append_term
    ├── commit_term
    ├── pre_append_term
    ├── prepare_term
    ├── retreive_n_term
    ├── retreive_term
    └── switch_status
```
- _append_term_: A new term has to be applied. This might be volatile and you
  may apply multiple times the same term. That's up to the user to manage his
  own logic with that behavior. It takes 2 arguments, the term id and the
  content. It doesn't have to dump anything on the standard output. In case of
  failure, if you're a follower, remote leader will receive an error, if you're
  a leader, you'll turn in idle and start a candidature.
- _commit_term_: The term is considered as definitive by the current leader.
  Append once. It takes 2 arguments, the term id and its content.
- _pre_append_term_: A term append from a potential leader but it has to pass the user checks.
  It takes 2 arguments, the id of the term and the content. To avoid gaps, the user should put
  in the standard output the `latest term id + 1`. The default behavior is to accept gaps and
  always print the first argument.
- _prepare_term_: If you are the leader, you can fill the terms by writing in
  the standard output there content. Hook cares about its id and its
  replication. As a leader, don't append the term now, wait the `append_term`
  call. Called each `prepare_term_period`
- _retrieve_term_: If you're a leader, that hook serves to rebuild a term which
  isn't in cache anymore. The terms to rebuild are supposed to be committed
  previously. It takes 1 argument, the term id. It expect to read the
  content of the term in the standard output. If the hook failed, the node
  turns in idle until the next election. The default behavior is to create
  a new "default" term (a term with default written in the content).
- _retrieve_n_term_: If Hook needs more than one term to rebuild, it will first
  try to use that one instead of the *retrieve_term* hook. It takes 2
  arguments, the begin and the end id. It expect to read on the
  standard output a JSON formatted list of terms
  with the format `[{'id':12,'content':'hello world'}]`.
- _switch_status_: Notification of all changes of status over the time, it
  takes one argument "candidate"|"follower"|"leader". It doesn't expect any
  output.

### Raft settings

When you start a node, you can target a settings file.

```toml
# Min and max value in milisecond of the election timeout. Timeout is randomly
# choosen between these two values.
timeout_min = 500
timeout_max = 800

# Value in milisecond that separe term preparations, default 80
# If this time is too short to finish the term preparation, an empty heartbeat
# will be send and the content will be used for the next term. The hook doesn't
# implement any problem management if you fail multiple times to send a term.
# You can manage it yourself with the `send-term` script
prepare_term_period = 80

# List of public known nodes in the network.
nodes = ['12.13.14.15:8080']

# Server local address, default "127.0.0.1"
addr = "127.0.0.1"
# Port used between nodes to communicate, default "3000"
port = "3000"

# If true, the current node will never ask for an election. Nevertheless you
# will receive all heartbeat and all information like a normal node. Some hooks
# will never be called obviously but you are a part of the network. If false,
# you will be considered as a potential candidate.
#
# default false
follower = false

# Value in millisecond before considering that a remote node will never respond
response_timeout = 200
```

## Run The node

That repository contains a rust library with all the tools to make a private
implementation. The basic implementation as simple as:

```Rust
use hook_raft::*;

/// Start a new node
fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Runtime expected to start but failed");
    match Node::new(DefaultHook {}).start(rt) {
        Ok(_) => println!("Successfully exit"),
        Err(err) => eprintln!("Node crash with error: {:?}", err),
    }
}
```

## Some information

- Hook nodes communication is over HTTP.
- Hook scripts have to be executable by the local user to work properly.
- The default binary is agnostic to the content of terms. The diffusion, the reason
  of why it's diffused, and the usage of the content is deferred to the user.
- Bootstrapping isn't managed. As well as the change of the cluster membership and
  the log compaction.

