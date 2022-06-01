# Listening the status modificatons

The status are those:
- pending (ConnectionPending)
- follower
- candidate
- leader

When the status change the local state also may change.

```mermaid
flowchart LR
    start(on `status := follower`)
    start_timeout[start a new heartbeat timeout]
    start --> start_timeout
```

```mermaid
flowchart LR
    start(on `status := candidate`)
    nothing(execute workflow\n`candidate_timeout`)
    start --> nothing
```

```mermaid
flowchart LR
    start(on `status := leader`)
    nothing(spawn leader thread)
    start --> nothing
```

# Allowed States Flow

The following graph describe the allowed states changes that can happen in the code. (Look a the `mod state`)

The only way to create a status in release is to use a `create` method and return a Status configured as ConnectionPending. If you want to get a follower or something else, you should follow the following graph steps.

```mermaid
flowchart
    ConnectionPending --> |on accepted by the leader\nlook at the leader_workflow| Follower
    ConnectionPending --> |"if follower == false && nodes is empty"| Leader
    Follower --> |if follower == false| Candidate
    Candidate --> Leader
    Candidate --> Follower
    Leader --> Follower
```