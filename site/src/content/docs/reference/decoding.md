---
title: ROS2 CDR decoding
description: How zenoh-mcp turns rmw_zenoh CDR payloads into JSON, and how to extend it.
---

ROS2's `rmw_zenoh_cpp` middleware publishes messages onto Zenoh with **CDR**
(Common Data Representation) payloads, under keys that encode the ROS type.
`zenoh-mcp` recognises those keys and deserialises the payloads to JSON.

## Key → type mapping

`rmw_zenoh` keys look like:

```
0/kuka/pose/geometry_msgs::msg::dds_::PoseStamped_/RIHS01_10f3786d7d40fd2b…
└┬┘ └──┬───┘ └──────────────┬──────────────────┘ └──────────┬───────────┘
domain  topic            type tag                       RIHS type hash
```

The server scans the key segments for the type tag pattern
`pkg::msg::dds_::Name_` and maps it to a ROS type name `pkg/Name`. For display, the
`key` field in results is shortened to just `domain/topic` (the type tag and RIHS
hash are dropped).

## Decoding pipeline

For each sample the payload is decoded as follows:

1. **Known ROS type** — if the inferred type has a registered decoder, the CDR
   body is deserialised into a typed struct and emitted as a JSON object.
2. **Recognised-but-unmapped ROS type** — if the key carries a type tag but no
   decoder exists (or decoding fails), the value is:
   ```json
   { "_ros_type": "pkg/Name", "_undecoded": true, "_bytes": 64, "_hex": "0001…" }
   ```
3. **Non-ROS payload** — if there is no type tag, the bytes are tried as UTF-8. If
   they parse as JSON, the parsed JSON is returned; otherwise the raw string is
   returned. Many non-ROS Zenoh apps publish JSON, so these surface cleanly.
4. **Opaque binary** — if none of the above apply:
   ```json
   { "_binary": true, "_bytes": 48, "_hex": "…" }
   ```

The `_hex` preview is capped at 48 bytes with an ellipsis.

## Built-in decoders

| ROS package | Types |
|-------------|-------|
| `std_msgs` | `String`, `Bool`, `Int32`, `Int64`, `Float32`, `Float64` |
| `geometry_msgs` | `Pose`, `Twist`, `PoseStamped`, `TwistStamped` |
| `sensor_msgs` | `JointState` |
| `rcl_interfaces` | `Log` |

These cover the common diagnostic/telemetry traffic on a ROS2 graph. The decoded
JSON matches what `ros2 topic echo` prints for the same message.

## Adding a type

Decoders live in [`src/decode.rs`](https://github.com/nargetdev/zenoh-mcp/blob/main/src/decode.rs).
Adding one is two steps:

1. Define a `struct` mirroring the ROS message, in **field order** (CDR is
   positional), deriving `Serialize, Deserialize`:

   ```rust
   #[derive(Debug, Serialize, Deserialize)]
   struct Imu {
       header: Header,
       orientation: Quaternion,
       angular_velocity: Vector3,
       linear_acceleration: Vector3,
   }
   ```

2. Add a match arm in `decode_known` keyed by the ROS type name:

   ```rust
   "sensor_msgs/Imu" => d!(Imu),
   ```

Field order must match the `.msg` definition exactly, including nested types and
`builtin_interfaces/Time`. Arrays/sequences map to `Vec<T>`, strings to `String`.

## Why CDR, and why this works

`rmw_zenoh` serialises with standard ROS2 CDR (little-endian, with the 4-byte
encapsulation header). The [`cdr`](https://crates.io/crates/cdr) crate reads the
encapsulation header and handles CDR alignment, so a `serde`-derived struct in the
right field order round-trips directly to JSON — no `.msg` parsing or code
generation required.
