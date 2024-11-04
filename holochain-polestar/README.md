# holochain-polestar

Examples of building Holochain with polestar patterns in mind.

# Ops

A model for how ops are propagated through a distributed network of nodes.

Each op has a finite state within each node, as it moves through the workflows, and certain states lead to ops being propagated to other nodes.
A useful model would be a focus on a single Op across all nodes in a network.

From this model:
1. we should be able to build up an actual network from that model.
2. we can observe actual behavior in the system and see if that constitutes valid behavior for the model.