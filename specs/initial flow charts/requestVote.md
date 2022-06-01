# On request vote receive

```mermaid
flowchart TD
    start(Receive an requestVote)
    script{{Script: request-vote}}
    success_1{success?}
    test_1{term\nindex received\n> local}
    test_2{votedFor == null\n&& lastLogIndex >= local}
    false[Reply false]
    update_state[Lock address has\npotential leader]
    true[Reply true]

    start --> script
    script --> success_1
    success_1 --> |false| false
    success_1 --> |true| test_1
    test_1 --> |false| false
    test_1 --> |true| test_2
    test_2 --> |true| update_state --> true
    test_2 --> |false| false
```

</br></br>

> In this workflow, we don't check if we are a leader or not, it's free to manage this behavior in another side module using clearly the hooks.
> You can also manage a `loyal` rule to prevent request spamming 
