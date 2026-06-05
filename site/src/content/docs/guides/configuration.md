---
title: Configuration
description: How zenoh-mcp picks a router and how to target multiple routers.
---

`zenoh-mcp` is configured almost entirely per-call. The only global knob is the
default router endpoint.

## Choosing a router

Every tool accepts an optional `connect` argument — a Zenoh endpoint such as
`tcp/192.0.2.10:7447`. The endpoint is resolved in this order:

1. The tool's `connect` argument, if provided.
2. The `ZENOH_CONNECT` environment variable.
3. The built-in default, `tcp/localhost:7447`.

This makes it easy to target **multiple routers** from one server instance: set a
sensible default in `ZENOH_CONNECT`, and override `connect` per call when you need
a different one.

```text
zenoh_list_keys { }                                  → uses ZENOH_CONNECT or localhost
zenoh_list_keys { "connect": "tcp/192.0.2.20:7447" } → that router instead
```

## Connection model

- The server always opens **client**-mode sessions, never peer or router mode. It
  attaches to the router you name and nothing else.
- Multicast scouting is disabled, so there is no LAN discovery traffic and no
  waiting on scout timeouts — connections to an explicit endpoint are immediate.
- Each tool call is **stateless**: open session → one operation → close. There is
  no long-lived subscription held between calls, which keeps the server safe to
  point at production routers.

## Endpoint formats

Zenoh endpoints follow `<protocol>/<address>[:port]`. Common ones:

| Endpoint | Meaning |
|----------|---------|
| `tcp/localhost:7447` | local router, default Zenoh port |
| `tcp/192.0.2.10:7447` | remote router by IP |
| `tcp/[::1]:7447` | IPv6 loopback |

## Logging

The server logs to **stderr** (stdout is reserved for the MCP JSON-RPC stream).
Control verbosity with `RUST_LOG`:

```bash
RUST_LOG=zenoh_mcp=debug,zenoh=info zenoh-mcp
```
