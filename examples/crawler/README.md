# Crawler

This small example serves both as an example for consuming the chainsync protocol, and a small utility you can customize for one-off crawling tasks.

By filling in the implementaiton of block_matches or tx_matches, you can easily save any blocks or txs that match some predicate.

The provided example saves any blocks that have a protocol update request, either at the block level (in byron eras) or the transaction level (in later eras). This was useful in acquiring test data from different environments for Amaru development, for example.

By replacing that predicate, or implementing the tx predicate, you can crawl the chain for your own needs.
