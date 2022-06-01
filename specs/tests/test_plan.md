# Testing hook

## Workflow testing

In the workflow module, you'll find a submodule `test` that call each functions of the workflow module with different input, verify that it succeed or fail with an Error or a Warning and check if the StatusPtr has correctly been updated after a success.

## Entire app testing

You'll find a script creating a test scenario, running in local a given number of node and check if the final result is the same for each terms.

Script Input parameters:
- N nodes: number of node that the script will run
- N followers: number of followers in nodes
- N known: Number of randomly choosen known nodes that can be leaders by nodes
- N stop: number of time the leaders will stop by herselfs
- N terms: number of terms the network will do before a stop

Configurations files will be created from the given input and a node will be launched for each one.

## Robustness face of attacks

This test is similar to the precedent one, but instead we will add a N number that correspond to a number of nodes that will be mocked and that will do a lot of random action. Another test add a number N of node that are only doing vote requests or only append_term, or only connection.

At the end of these tests, we check the integrity of the final datas cumulated for each node and produce a resume of the test.
 