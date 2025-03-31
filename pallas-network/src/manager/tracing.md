# Tracing Strategy

## Peer-Specific Spans
- Each peer should have its own span tree that represents its lifecycle
- This helps track individual peer behavior, connection states, and protocol interactions
- Useful for debugging specific peer issues
```
peer.{peer_id}
├── connect
├── handshake
├── protocol_interactions
│   ├── chain_sync
│   ├── block_fetch
│   └── tx_submission
└── disconnect
```

## Behavior-Specific Spans

```
behavior.{type}

```


## Manager-Level Spans
- Higher-level spans that show the manager's operations across all peers
- Helps understand system-wide behavior and coordination
- Useful for monitoring overall network health

```
network_manager
├── sprint
```