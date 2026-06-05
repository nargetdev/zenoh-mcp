---
title: Getting started
description: Build zenoh-mcp and register it with an MCP client.
---

`zenoh-mcp` is a Rust binary. Build it once, then point your MCP client at it.

## Prerequisites

- A Rust toolchain (`cargo`), stable.
- Network reachability to a Zenoh router (default `tcp/localhost:7447`). The
  binary connects in **client** mode — it does not run its own router.

## Build

```bash
git clone https://github.com/nargetdev/zenoh-mcp
cd zenoh-mcp
cargo build --release
# binary at ./target/release/zenoh-mcp
```

The first build compiles `zenoh 1.8.0` and `rmcp`, so it takes a couple of
minutes. Subsequent builds are fast.

## Register with Claude Code

```bash
claude mcp add zenoh -- /abs/path/to/zenoh-mcp/target/release/zenoh-mcp

# or pin a default router via env var:
claude mcp add zenoh \
  -e ZENOH_CONNECT=tcp/192.0.2.10:7447 \
  -- /abs/path/to/zenoh-mcp/target/release/zenoh-mcp
```

Verify it connected:

```bash
claude mcp list   # zenoh: ... ✓ Connected
```

## Register with any MCP client

Add a stdio server entry (e.g. `.mcp.json`, `claude_desktop_config.json`, or your
client's equivalent):

```json
{
  "mcpServers": {
    "zenoh": {
      "command": "/abs/path/to/zenoh-mcp/target/release/zenoh-mcp",
      "env": { "ZENOH_CONNECT": "tcp/192.0.2.10:7447" }
    }
  }
}
```

## First calls

1. `zenoh_info` — confirm the binary can reach your router.
2. `zenoh_list_keys` — discover what is being published (like `ros2 topic list`).
3. `zenoh_subscribe` with a `key` from the list — tap it for a few seconds and
   read decoded samples.

See the [Tool reference](/zenoh-mcp/reference/tools/) for every argument.
