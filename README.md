# zenoh-mcp

A small **stdio [MCP](https://modelcontextprotocol.io) server** that gives AI agents
hands-on **Zenoh debugging tools** — query, publish, tap topics, and discover keys
on a [Zenoh](https://zenoh.io) network, with **ROS2 (`rmw_zenoh`) CDR payloads decoded
to JSON** automatically.

Pinned to **`zenoh = 1.8.0`** so it is wire-compatible with a 1.8.0 router and with
ROS2 Kilted's `rmw_zenoh_cpp`. No Python, no pip — a single static Rust binary.

> Built because, as of mid-2026, there is no common/mature Zenoh MCP server.
> The only comparable effort is the (unreleased, dormant) `gabrik/zenoh-plugin-mcp`,
> a heavyweight `zenohd` plugin over HTTP/SSE. This is the opposite: a lightweight
> stdio tool focused on *debugging an existing network*.

## Tools

| Tool | What it does |
|------|--------------|
| `zenoh_info` | Connect to a router and report our Zenoh ID + reachable router admin keys. Confirm reachability first. |
| `zenoh_get` | Send a GET for a selector. Works for data queryables and **router admin** (`@/**` → topology, plugins, transports). CDR replies decoded. |
| `zenoh_put` | Publish a UTF-8 value to a key expression. Poke subscribers/queryables. |
| `zenoh_subscribe` | Tap a key expression for N seconds and return the samples, **ROS2 CDR decoded to JSON**. *"What's being published on this topic right now?"* |
| `zenoh_list_keys` | Listen briefly and return distinct keys, each with inferred ROS2 type + sample count. Like `ros2 topic list` for the Zenoh wire. |

Every tool is **stateless**: it opens a short-lived *client*-mode session to the
target router, does one operation, and closes — safe to point at any router.

### Choosing the router

Each tool takes an optional `connect` argument (e.g. `tcp/localhost:7447`).
If omitted, it falls back to the `ZENOH_CONNECT` env var, then to
`tcp/localhost:7447`.

## ROS2 decoding

`rmw_zenoh` keys carry a type tag like `0/kuka/pose/geometry_msgs::msg::dds_::PoseStamped_/RIHS01_…`.
The server maps that to `geometry_msgs/PoseStamped` and deserializes the CDR body.
Decoders are included for the common types (`std_msgs/{String,Bool,Int32,Int64,Float32,Float64}`,
`geometry_msgs/{Pose,Twist,PoseStamped,TwistStamped}`, `sensor_msgs/JointState`,
`rcl_interfaces/Log`). Unknown CDR types return `{_ros_type, _undecoded, _bytes, _hex}`;
plain JSON/text payloads (non-ROS) are surfaced as-is. Adding a new type is one
struct + one match arm in [`src/decode.rs`](src/decode.rs).

## Build

```bash
cargo build --release
# binary: ./target/release/zenoh-mcp
```

## Use with Claude Code

```bash
claude mcp add zenoh -- /path/to/zenoh-mcp/target/release/zenoh-mcp
# optionally pin a default router:
claude mcp add zenoh -e ZENOH_CONNECT=tcp/localhost:7447 -- /path/to/zenoh-mcp/target/release/zenoh-mcp
```

Or any MCP client (`.mcp.json` / `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "zenoh": {
      "command": "/path/to/zenoh-mcp/target/release/zenoh-mcp",
      "env": { "ZENOH_CONNECT": "tcp/localhost:7447" }
    }
  }
}
```

## Example

`zenoh_subscribe { "key": "0/kuka/joint_states/**", "duration_secs": 2 }`:

```json
{
  "key": "0/kuka/joint_states",
  "kind": "Put",
  "ros_type": "sensor_msgs/JointState",
  "value": {
    "header": { "frame_id": "kuka_base", "stamp": { "sec": 1780658057, "nanosec": 215979475 } },
    "name": ["a1","a2","a3","a4","a5","a6"],
    "position": [-0.140, -1.582, 1.779, 0.135, 0.809, -0.248],
    "velocity": [], "effort": []
  }
}
```

## License

MIT
