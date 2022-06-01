# Connection workflow


## Connection (your side)

```mermaid
flowchart TD
    start
    pending{{status := connection pending}}
    server["Start the server\nlocalhost:{config_port}"]
    conf[Collect configuration]
    checkNodes{known nodes?}
    warning[Print warning]
    update[Update server local State]
    connect{Connection:\nupdateNode}
    is_leader{Updated\nfrom\nleader data?}
    send_leader[Send 'updateNode' to leader]
    success_1[success]
    leader{{status := leader}}
    wait["Start the server\nlocalhost:{config_port}"]
    build_request[Build the updateNode request]
    is_follower{Configured\nas follower}

    start --> pending
    pending --> conf
    conf --> server
    server --> build_request
    build_request --> checkNodes
    connect --> |Try for each nodes| connect
    checkNodes --> |yes| connect
    checkNodes --> |no| warning
    warning --> is_follower
    connect --> |success| update
    update --> is_leader
    is_leader --> |yes| success_1
    is_leader --> |no| send_leader
    send_leader --> success_1
    send_leader --> |fail| Error
    success_1 --> us{{status := ConnectionPending}}
    us --> w[wait for leader connection accept]
    connect --> |fail| Error
    is_follower --> |yes| Error
    is_follower --> |no| leader
```

> Local server state is updated with the response of the updateNode request.
> - `leaderId` so follower can redirect clients
> - `leaderCommit` leader’s `commitIndex`
</br></br>

</br>

---

## On receive connection request (network side)
```mermaid
flowchart TD
    on_update_node(Receive updateNode)
    is_leader{Am I the\nleader?}
    add_in_pool[Add address to a pool]
    update{{Script: update-node}}
    update_success{success?}
    remove[Remove from known]
    ok(return ok)
    nok(dismiss)
    on_error[/on error/]
    is_known{is known?}

    on_update_node --> update
    update --> update_success
    update_success --> |yes| is_leader
    on_error --> nok
    update_success --> |no| nok
    is_leader --> |yes| is_known
    is_known --> |yes| remove
    remove --> add_in_pool
    is_known --> |no| add_in_pool
    add_in_pool --> ok
    is_leader --> |no| ok
```

> The pool of connection request is managed in the same thread than the send of terms. An appendTerm should be send with a special information for the network. This information permit to the network to add the new address. **The sender of the request receive a special term when the previous is commited**.

</br>

---

## Receiving connection accepted (your side)

The first term that your node receive means that your node has been accepted and commited in the network.

```mermaid
flowchart TD
    st{{status == pending}}
    start(Receive an appendTerm)
    update[Update state]
    follower{{status := follower}}
    
    st{{status == pending}} --o start
    start --> update
    update --> follower
```
