---
title: MCP server spec
description: Protocol, transport, capabilities, and conventions of the zenoh-mcp server.
---

This page specifies the `zenoh-mcp` server as an MCP implementation: its
transport, declared capabilities, tool surface, and shared conventions. For the
per-tool argument and output detail, see the [Tool reference](/zenoh-mcp/reference/tools/).

## Identity

| Field | Value |
|-------|-------|
| Server name | `rmcp` (from the `rmcp` SDK build env) |
| Implementation | `zenoh-mcp` |
| Protocol version | `2024-11-05` |
| Transport | stdio (newline-delimited JSON-RPC 2.0) |
| Zenoh version | `1.8.0` (pinned) |
| SDK | [`rmcp`](https://crates.io/crates/rmcp) 1.7 |

## Capabilities

The server declares **tools** only. It does not expose prompts, resources, or
sampling.

```json
{ "capabilities": { "tools": {} } }
```

## Transport

- The process reads JSON-RPC requests on **stdin** and writes responses on
  **stdout**, one JSON object per line.
- All diagnostic logging goes to **stderr** and never pollutes the protocol
  stream. Set verbosity with `RUST_LOG` (e.g. `zenoh_mcp=debug`).
- The server runs until stdin closes (the client disconnects).

## Handshake

Standard MCP initialization:

1. Client → `initialize` with its `protocolVersion` and `clientInfo`.
2. Server → `InitializeResult` with capabilities and instructions.
3. Client → `notifications/initialized`.
4. Client → `tools/list`, then `tools/call`.

## Tool surface

| Tool | Summary |
|------|---------|
| `zenoh_info` | Connect to a router; report our Zenoh ID and reachable router-admin keys. |
| `zenoh_get` | Send a GET (query) for a selector; return replies (data or router admin). |
| `zenoh_put` | Publish a UTF-8 value to a key expression. |
| `zenoh_subscribe` | Subscribe to a key expression for N seconds; return decoded samples. |
| `zenoh_list_keys` | Listen briefly; return distinct keys with inferred ROS2 type and counts. |

## Shared conventions

### Router selection

Every tool takes an optional `connect` string. Resolution order:
`connect` argument → `ZENOH_CONNECT` env var → `tcp/localhost:7447`. See
[Configuration](/zenoh-mcp/guides/configuration/).

### Statelessness

Each call opens a fresh **client**-mode Zenoh session, performs exactly one
operation, and closes it. No subscription or session state persists across calls.

### Bounded execution

Time- and size-bounded tools clamp their inputs so a call cannot run unbounded:

| Argument | Default | Clamp |
|----------|---------|-------|
| `timeout_ms` (`zenoh_get`) | 3000 | ≤ 30000 |
| `duration_secs` (`subscribe`, `list_keys`) | 3 | 1–30 |
| `max_samples` (`subscribe`) | 100 | 1–2000 |

### Output format

Every tool returns a single text content block containing a **pretty-printed JSON
object**. Sample- and reply-bearing tools wrap results in a common record shape:

```json
{
  "key": "0/kuka/pose",
  "ros_type": "geometry_msgs/PoseStamped",
  "kind": "Put",
  "value": { "...": "decoded payload or fallback" }
}
```

- `key` — the key expression, with the ROS2 type tag and RIHS hash suffix stripped
  for readability.
- `ros_type` — the ROS2 message type inferred from the key, or `null` for non-ROS
  keys.
- `kind` — the Zenoh sample kind (`Put` / `Delete`).
- `value` — the decoded payload. See [ROS2 CDR decoding](/zenoh-mcp/reference/decoding/)
  for how `value` is produced and its fallbacks.

### Errors

Operational failures (cannot open session, bad selector, etc.) are returned as
MCP `internal_error` responses with a message string. A query that simply yields
no replies is **not** an error — it returns `count: 0`.

## Security considerations

- The server can **read from and publish to** any router it can reach. `zenoh_put`
  mutates network state. Treat access to this server as equivalent to direct
  access to the target router.
- There is no built-in key-expression allow/deny filter; scope access by
  controlling which `ZENOH_CONNECT` / `connect` endpoints are reachable from the
  host running the server.
- The default endpoint is `tcp/localhost:7447`; nothing is contacted until a tool
  is called.
