---
title: Tool reference
description: Arguments and output for every zenoh-mcp tool.
---

Every tool returns a single JSON object (as a text content block). The `connect`
argument is optional everywhere and resolves to `connect` → `ZENOH_CONNECT` →
`tcp/localhost:7447`.

## `zenoh_info`

Connect to a router and confirm reachability.

**Arguments**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `connect` | string | no | Router endpoint. |

**Returns**

```json
{
  "connected": true,
  "endpoint": "tcp/192.0.2.10:7447",
  "session_zid": "3e68be0d2e4cfd72e6cdc6fe26cc998c",
  "zenoh_version": "1.8.0",
  "router_admin_keys": ["@/<zid>/router"]
}
```

## `zenoh_get`

Send a Zenoh GET (query) for a selector and return replies. Works for data
queryables and for the **router admin space** (`@/**` → topology, plugins,
transports). CDR replies are decoded.

**Arguments**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `selector` | string | **yes** | — | Selector / key expression, e.g. `@/**` or `demo/**`. |
| `connect` | string | no | — | Router endpoint. |
| `timeout_ms` | integer | no | 3000 (≤30000) | Max time to wait for replies. |

**Returns**

```json
{
  "selector": "@/**",
  "endpoint": "tcp/192.0.2.10:7447",
  "count": 1,
  "replies": [
    { "key": "@/<zid>/router", "ros_type": null, "kind": "Put", "value": { "...": "..." } }
  ]
}
```

A reply that carries a query error becomes `{ "error": "<message>" }`.

## `zenoh_put`

Publish a UTF-8 value to a key expression. Useful for poking subscribers and
queryables. **Mutates network state.**

**Arguments**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `key` | string | **yes** | Key expression to publish to, e.g. `demo/test`. |
| `value` | string | **yes** | UTF-8 payload. |
| `connect` | string | no | Router endpoint. |

**Returns**

```json
{ "ok": true, "key": "demo/test", "bytes": 11, "endpoint": "tcp/192.0.2.10:7447" }
```

## `zenoh_subscribe`

Subscribe to a key expression for a few seconds and return the samples received,
with ROS2 CDR payloads decoded to JSON. The primary tool for *"what is being
published on this topic right now?"*.

**Arguments**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `key` | string | no | `**` | Key expression to subscribe to. |
| `connect` | string | no | — | Router endpoint. |
| `duration_secs` | integer | no | 3 (1–30) | How long to collect samples. |
| `max_samples` | integer | no | 100 (1–2000) | Stop early after this many samples. |

**Returns**

```json
{
  "key": "0/kuka/joint_states/**",
  "endpoint": "tcp/192.0.2.10:7447",
  "duration_secs": 2,
  "count": 1,
  "truncated": true,
  "samples": [
    {
      "key": "0/kuka/joint_states",
      "ros_type": "sensor_msgs/JointState",
      "kind": "Put",
      "value": {
        "header": { "frame_id": "kuka_base", "stamp": { "sec": 1780658057, "nanosec": 215979475 } },
        "name": ["a1", "a2", "a3", "a4", "a5", "a6"],
        "position": [-0.140, -1.582, 1.779, 0.135, 0.809, -0.248],
        "velocity": [],
        "effort": []
      }
    }
  ]
}
```

`truncated` is `true` when collection stopped because `max_samples` was reached
rather than the duration elapsing.

## `zenoh_list_keys`

Discover topics: listen briefly on a key expression and return the distinct keys
seen, each with its inferred ROS2 type and a sample count. Like `ros2 topic list`
for the Zenoh network.

**Arguments**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `key` | string | no | `**` | Key expression to sweep. |
| `connect` | string | no | — | Router endpoint. |
| `duration_secs` | integer | no | 3 (1–30) | How long to listen before reporting. |

**Returns**

```json
{
  "endpoint": "tcp/192.0.2.10:7447",
  "duration_secs": 3,
  "distinct_keys": 2,
  "keys": [
    { "key": "0/chatter", "ros_type": "std_msgs/String", "samples": 3 },
    { "key": "0/kuka/pose", "ros_type": "geometry_msgs/PoseStamped", "samples": 83 }
  ]
}
```
