# kitsune-model

Polestar models of kitsune2 gossip

## Properties

- [ ] ensure that you always eventually gossip with every node you know about
  - [ ] ensure that on error, you attempt gossip again after N ticks
- [ ] don't gossip with a node you've already successfully gossiped with in the last tick
- [ ] don't gossip with a node you've already unsuccessfully gossiped with in the last N ticks