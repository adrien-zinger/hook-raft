# Hook / Raft

Hook is an implementation of the raft algorithm and want to divide the work of the node from the raft logic (inspiration from hooks in git).

## _Principia raftica_

The raft algorithm works with terms, and that's what hook does. But by default the terms are empty because it doesn't have to be important in a first place. Even if you'll be able to hack this project and fills terms, the main target of hook is to know who's the leader in the network.

So Hook will connect all nodes of the network (accepting new nodes with a bootstrap strategy) and send basic pings as terms for the raft.

## Hook? 🪝

That's all? Noooo, I called the project Hook because it implements a logic as the hooks in git repository. Actually, you can see that your node is linked to a `hooks` folder where you can see:

```bash
└── hooks
    ├── pre-append-term.sample
    ├── post-append-term.sample
    ├── apply-term.sample
    ├── commit-term.sample
    ├── leader-change.sample
    ├── pre-add-connection.sample
    ├── post-add-connection.sample
    ├── remove-connection.sample
    ├── lost-connection.sample
    ├── request-vote.sample
    ├── receive-vote.sample
    ├── prepare-term.sample
    ├── send-term.sample
    ├── terms-list.sample
    └── update-node.sample
```

Each script is called in a specific moment inside the raft algorithm if present in the folder. Extension allowed to are `*.sh` and `*.py` (ran with the simple `python` command).

Define inside the script how do you handle the event.

-   _pre-append-term_: A term append from the leader, the result of the script has to be "true", otherwise the term is rejected (Hook accepts all term by default)
-   _post-append-term_: A term append from the leader and so you can handle it
-   _apply-term_: You have a term with a content and you want to apply something to a state machine or something? It's here
-   _leader-change_: A new leader win the election
-   _pre-add-connection_: A new node want to join the network, the return of the script has to be "true" you accept the connection. All other response is considered has rejected. (Look at [default hook requirement](#default-hook-node-requirements))
-   _post-add-connection_: A connection has been added in the network
-   _remove-connection_: A node has been removed from the network
-   _lost-connecton_: A node as failed to answer, or the leader failed to send heartbeat
-   _request-vote_: Vote requested for a new leader
-   _receive-vote_: Received a response for your personal vote request if you are a candidate
-   _prepare-term_: If you are the leader, you can fill the content with the of `terms` with this script, the returned value is considered as the full raw content. This script is called `PREPARING_OFFSET` ms before each heartbeat.
-   _send-term_: It's call when you're a leader, and you send a term, you can check if you successfully prepare the last term here
-   _terms-list_: Return a list with an offset and a length, an attribute of terms.
-   _update-node_: A node status just changed (append when a bootstrap has been finalized by a node and became a potential candidate)

The status of the implementation is watched with the `config` crates and is updated on save in an acceptable time.

## RPC API

```rust
#[rpc(server)]
pub trait Rpc {
    #[rpc(name = "requestVote")]
    fn request_vote(&self, input: RequestVoteInput) -> Result<RequestVoteResult>;

    #[rpc(name = "appendTerm")]
    fn append_term(&self, input: AppendTermInput) -> Result<AppendTermResult>;

    #[rpc(name = "updateNode")]
    fn update_node(&self, input: UpdateNodeInput) -> Result<UpdateNodeResult>;
}
```

A raft node just need these 3 minimal function to work. Actually, a true raft node just require two of them (request*vote & append_term). The next RPC are used for the `bootstrap` feature. The other entries are described in the [raft specification](./specs/raft.pdf) by \_Diego Ongaro and John Ousterhout*. But let's write again what is said!

### append term

Invoked by leader to replicate log entries (§5.3); also used as heartbeat (§5.2).
Arguments:

-   `term` leader’s term
-   `leaderId` so follower can redirect clients
-   `prevLogIndex` index of log entry immediately preceding new ones
-   `prevLogTerm` term of `prevLogIndex` entry
-   `//entries[] log entries to store (empty for heartbeat; may send more than one for efficiency)` not implemented in hook design
-   `leaderCommit` leader’s `commitIndex`

Results:

-   `term == currentTerm`, for leader to update itself
-   `success == true` if follower contained entry matching
-   `prevLogIndex` and `prevLogTerm`

Receiver implementation:

1. Reply `false` if `term < currentTerm` (§5.1)
2. Reply `false` if log doesn’t contain an entry at `prevLogIndex` whose term matches `prevLogTerm` (§5.3) (not implemented in hook, can be using the `pre-append-term`)
3. (Hook implementation) Call `pre-append-term`
4. If an existing entry conflicts with a new one (same index but different terms), delete the existing entry and all that follow it (§5.3) (Not implemented in hook, can be done in `pre-append-term` or `post-append-term`)
5. Append any new entries not already in the log (Not implemented in hook, can be done in `pre-append-term` or `post-append-term`)
6. If `leaderCommit > commitIndex`, set `commitIndex = min(leaderCommit, index of last new entry)`
7. (Hook implementation) Call `post-append-term`

### Request vote

Invoked by candidates to gather votes (§5.2). If you receive this, you're also a potential candidate.
Arguments:

-   `term` candidate’s term
-   `candidateId` candidate requesting vote
-   `lastLogIndex` index of candidate’s last log entry (§5.4)
-   `lastLogTerm` term of candidate’s last log entry (§5.4)

Results:

-   `term == currentTerm`, for candidate to update itself
-   `voteGranted == true` means candidate received vote

Receiver implementation:

1. Reply `false` if `term < currentTerm` (§5.1)
2. If `votedFor` is `null` or `candidateId`, and candidate’s log is at least as up-to-date as receiver’s log, grant vote (§5.2, §5.4)
3. (Hook implementation) Call `request-vote`

### Update node

Invoked by new nodes when attempt to (re)connect to the network.
Arguments:

-   node type (follower or potential candidate)

Results:

-   `leaderId` so follower can redirect clients
-   `leaderCommit` leader’s `commitIndex`

Receiver implementation:

1. Re root the message to the current leader, if fail, reject the connection and ask for a retry. (the node can start bootstrapping)
2. The leader receive the connection request
3. Call `pre-add-connection`, if return true, continue
4. Check if the bootstrap strategy of the node is the same as the receiver strategy, if true, continue
5. Propagate a new connection request to all server with a new term
    - When engaged, signal to the connecting node the current list of nodes
    - For the node that attempted to connect, update the list of nodes and followers (lists contained in the term)

When the node require being a candidate, he needs to finish the bootstrap first. The design to verify if the bootstrap has been correctly done isn't implemented in Hook but can be done with the hooks scripts, thanks to your own imagination. That's said, the update of the node from followers to node follow the same process that described above.

## Server Raft

### Servers status

The server keep a static status with network information. Still with the [raft specification](./specs/raft.pdf) with the same chapter, I can define what is in the status:

For all server: _(Updated on stable storage before responding to RPC)_

-   `currentTerm` the latest term server has seen (initialized to 0 on first boot, increases monotonically)
-   `votedFor` `candidateId` that received vote in current term (or null if none)
-   `//log[] log entries;` not implemented in Hook
-   nodes[] keep all known server that can also being candidates and considered for votes
-   followers[] list all nodes that are only followers, cannot be candidate and leader and cannot vote. These nodes are maybe currently bootstrapping or don't respect the server requirements. It can be voluntary set in the node config.

Volatile state on all servers:

-   `commitIndex` index of the highest log entry known to be committed (initialized to 0, increases monotonically)
-   `lastApplied` index of the highest log entry applied to state machine (initialized to 0, increases monotonically)

Volatile state on leaders: _(Reinitialized after election)_

-   `nextIndex[]` for each server, index of the next ~~log~~ entry to send to that server (initialized to leader last ~~log~~ entry index + 1)
-   `matchIndex[]` for each server, the index of the highest ~~log~~ entry known to be replicated on the server (initialized to 0, increases monotonically)

### Servers rules

All Servers:

-   If `commitIndex > lastApplied`: increment `lastApplied`, call `apply-term`
-   If RPC request or response contains term `T > currentTerm` then set `currentTerm = T` and convert to follower (§5.1)

Followers (§5.2):

-   Respond to RPC from candidates and leaders
-   If election timeout elapses without receiving `appendTerm` RPC from current leader or granting vote to candidate: convert to candidate

Candidates (§5.2):

-   On conversion to candidate, start election:
-   Increment `currentTerm`
-   Vote for self
-   Reset election timer
-   Send `requestVote` RPC to all other servers
-   If votes received from the majority of servers: become leader
-   If `appendTerm` RPC received from new leader: convert to follower
-   If election timeout elapses: start new election

Leaders:

-   Upon election: send initial empty `appendTerm` RPC (heartbeat) to each server; repeat during idle periods to prevent election timeouts (§5.2)
-   `//If command received from client: append entry to local log, respond after entry applied to state machine (§5.3)` this is not implemented in Hook. Instead, you have to locally create a context outside the Hook implementation in your own software and manage entries as you want to. Then you can use the `prepare-term` script to pop your entries as you want to do. You're free to wrap multiple entries in a term!
-   If last log `index ≥ nextIndex` for a follower: send `appendTerm` RPC with log entries starting at `nextIndex`
    -   If successful: update `nextIndex` and `matchIndex` for follower (§5.3)
    -   If fails because of log inconsistency: decrement `nextIndex` and retry (§5.3)
-   If there exists an N such that `N > commitIndex`, a majority of `matchIndex[i] ≥ N`, and `log[N].term == currentTerm`: set `commitIndex = N` (§5.3, §5.4).

### Heartbeat and election

The raft consensus algorithm mainly work with timeouts and heartbeat.

1. Non-leaders & non-followers nodes have all an election timeout that is incremented each time he receives a term from the current leader. And when the timeout reach 0, the node convert itself to a candidates. (see the [servers rules section](#servers-rules))

2. Leaders have a heartbeat choosen by the ownerof the server with the settings. This heartbeat is a timeout loop and define when a leader send terms to other nodes in the network. The second leader timeout is set after each propagation of terms, and is shorter than the heartbeat, it's an offset that say to the node when he should prepare his term to send.

```markdown
> Illustration of the leader timeouts

             Term thread               Main thread
                x                            |

_20ms!_ | |
start the | |
term prep | push the new term in |
-aration | a queue |
|----->|--------------------------->|
x |
x |
_60ms!_ x | - Pop and send the term to the network
x | If the queue is empty, we send an empty term
x | _And we reset the heartbeat_ & _reset offseted heartbeat_
x |
_20ms!_ start | |
the term prep | |
-aration | |
:-( to slow | |
| |
_60ms!_ | | - Pop and send... but the term thread is steal working.
| | Just _reset the heartbeat_ and notify the problem in
| | the `send-term` script.
Term prepared! | Push term... |
|---->|--------------------------->|
x |
x |
. .
. .
. .
```

### Raft settings

When starting a new node, you can target a settings file. Note that you shouldn't run a node on a network with random settings because you may fail to connect.

```toml
timeout_min = 150
timeout_max = 300
# Min and max value in milisecond of the election timeout. Timeout is randomly choosen between these two values

heartbeat = 60
# Value in milisecond that separe heartbeats

prepare_term_offset = 20
# Offset in milisecond used to prepare the term before the heartbeat, it has to
# be lower than the heartbeat of course.
# If this time is too short to finish the term preparation, an empty heartbeat
# will be send and the content will be used for the next term. The hook doesn't
# implement any problem management if you fail multiple times to send a term.
# You can manage it yourself with the `send-term` script

nodes = ['12.158.20.36']
# Optional list of public known nodes in a network. If this list appear to be empty, the node won't connect to anything and will be the current leader of his own network.

followers = ['15.126.208.72']
# Optional list of known followers

addr = "0.0.0.0"
# Server local address, default "0.0.0.0"
port = "12589"
# Port used between nodes to communicate, default "12589"

follower = true
# If true, the current node will never ask for an election and will never be able to vote. Nevertheless you will receive all heartbeat and all information like a normal node. Some hooks will never be called obviously but you are a part of the network. If false, you will be considered as a potential candidate after a successfull bootstrap and will be able to vote.

response_timeout = 20
# Value in millisecond before considering that a node will never respond
```

## Starting a node

This section describe how a node will works. The behavior is different if you're trying to be a candidate in the network.

Behavior of your node and the network when you try to follow.

1. The node will try to connect to each known nodes (see the [settings section](#raft-settings))
    - The connections are done with an `updateNode` RPC request
    - If success, update the local informations, know you should know the leader address
    - If failed and you are just a follower, the node will stop with an error. Otherwise you're the new leader of your kingdom
2. The node will send again (if not already sent) an `updateNode` request to the leader.
    - The leader is informed that you want to connect, you can wait quitly to be added inside the network
3. The leader will add your address in a pool of address to verify.
    - The verification is done in the `update-node` script (see the [first section](#hook?-🪝))
    - If you success to pass the verification of the leader, you'll be added inside the known nodes or followers, otherwise nothing happen and skip the next actions. Note that inside the script you can define a boostrap verification behavior or limit access to addresses in a mask (see also the [bootstrap node](#bootstrapping))
    - If the leader receive a second time the address to add before checking it, he'll ignore the command
4. The node will send to the potential candidates in the network the new address to add with a special term.
    - The leader will prepare a special term with a command to insert the address
    - The leader choose randomly if send a classical term and a special term
5. When this term is commited, the leader will start sending terms to the node.
    - If you're a potential candidate, you should start an election timeout

If you create a node that want to be a candidate and you fail to connect with all the boostrap nodes in the list, the node will automatically be a leader. So the first node of the network (the older) has the priviledge to be a leader.

## Default Hook Node Requirements

Hook accepts all connection by default if requirements are ok.

Rejected causes:

-   The connecting node has the another bootstrap strategy than the bootstrap target node
-   The node want to be candidate but failed to communicate with another randomly chosen node in the follower list (I don't know if it's really useful)
-   One of the nodes rejected your connection

## Snob a node 🧐

A node is locally temporary ban if it takes too much time to respond. We'll consider that the node has been disconnected and require a new call of `updateNode` RPC. If a node attempts to request a vote when it has been flagged as banished, we don't vote for him, we don't accept terms, and he needs to update itself to return to a normal status.
When a node is banished, the leader send a term that said to change the node status in all servers that can be or will be able to candidate.

## Bootstrapping

The bootstrap system isn't managed here. If you want to implement a bootstrap strategy, you can develop it your own server who use hook and check in the `update-node` script if the node who attempt to connect fill requirements.
