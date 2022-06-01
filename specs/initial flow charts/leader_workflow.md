# Leader workflow

The leader workflow is a loop in a dedicated thread. The thread manage with an *address pool*, some *hook scripts* and send terms in the network. It doesn't manage the status changes, the vote requests and the append term request. The worker thread is spawned only when you became a leader.

The workflow use the words queue or pool, it's the same thing and just means that it interact with a global variable accessible between threads. For the *addresses* it's mean a list of `updateNode` requests to handle and manage. By *terms queue*, it means a list of terms raw prepared filled by the leader and sent by the leader.

Scripts are performed asynchronously with concurrency in the thread worker.


</br></br>

---
## On start

```mermaid
flowchart TD
    start(On spawn thread)
    heartbeat_term[Start the heartbeat\nterms timeout]
    prepare_term_timeout[Start the heartbeat\nterms timeout]

    start --> heartbeat_term
    start --> prepare_term_timeout
```

</br></br>

---

## On preparation timeout

```mermaid
flowchart TD
    start(on term preparation timeout)
    success{success}
    push[push term in the queue]
    finish[End]

    start --> script_prepare_term
    script_prepare_term --> success
    success --> |yes| push
    success --> |no| finish
    push --> finish

    script_prepare_term{{Script: prepare-term}}
```


</br></br>

---

## On heartbeat timeout

```mermaid
flowchart TD
    start(on heartbeat term timeout)
    pop_terms{pop terms raw}
    pop_address[pop address update task]
    connect_required{Update node required}
    random{randomly choose\nbetween update node\nand term pool}
    build[build term]
    build_empty[build empty term]
    success_send{success?}
    send(send term for each follower)
    send_commited[update commit index]
    increment[increment negative\nresponse counter]
    finish[Restart the two timeouts]
    response{response index\n> local index}
    test_negativ{negative count\n< 50%}
    follower{{status := follower}}
    script_send_term{{Script: send-term}}
    apply{{Script: apply-term}}
    shuffle[Shuffle list of nodes]
    previous{Term to apply}
    connect_required_info[/If one node asked to\nbe updated look at the update address pool\n& init workflow/]

    start --> previous --> |yes| apply --> connect_required
    connect_required_info --o connect_required
    previous --> |no| connect_required
    connect_required --> |yes| random
    random --> |term| pop_terms
    random --> |address| pop_address
    pop_address --> build
    connect_required --> |no| pop_terms
    pop_terms --> |term found| build
    build --> script_send_term
    pop_terms --> |empty| build_empty
    build_empty --> script_send_term
    script_send_term --> success_send

    
    success_send --> |yes| shuffle --> send
    test_negativ --> |true| send_commited
    test_negativ --> |false| follower
    
    success_send --> |no| finish
    send --o |for each| response
    response --> |yes| increment

    send --o |sent to all| test_negativ
    send_commited --> finish
```
