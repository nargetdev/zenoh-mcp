//! ROS2 (rmw_zenoh) CDR payload decoding.
//!
//! rmw_zenoh keys carry a type tag like `pkg::msg::dds_::Name_`. We map that to
//! a `pkg/Name` ROS type and deserialize the CDR body into JSON. Unknown CDR
//! types and plain (JSON/text) payloads are handled with graceful fallbacks.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ---- ROS2 message definitions (field order == CDR wire order) ----

#[derive(Debug, Serialize, Deserialize)]
struct Time {
    sec: i32,
    nanosec: u32,
}
#[derive(Debug, Serialize, Deserialize)]
struct Header {
    stamp: Time,
    frame_id: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct StringMsg {
    data: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct Bool {
    data: bool,
}
#[derive(Debug, Serialize, Deserialize)]
struct Int32 {
    data: i32,
}
#[derive(Debug, Serialize, Deserialize)]
struct Int64 {
    data: i64,
}
#[derive(Debug, Serialize, Deserialize)]
struct Float32 {
    data: f32,
}
#[derive(Debug, Serialize, Deserialize)]
struct Float64 {
    data: f64,
}
#[derive(Debug, Serialize, Deserialize)]
struct Point {
    x: f64,
    y: f64,
    z: f64,
}
#[derive(Debug, Serialize, Deserialize)]
struct Quaternion {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}
#[derive(Debug, Serialize, Deserialize)]
struct Pose {
    position: Point,
    orientation: Quaternion,
}
#[derive(Debug, Serialize, Deserialize)]
struct PoseStamped {
    header: Header,
    pose: Pose,
}
#[derive(Debug, Serialize, Deserialize)]
struct Vector3 {
    x: f64,
    y: f64,
    z: f64,
}
#[derive(Debug, Serialize, Deserialize)]
struct Twist {
    linear: Vector3,
    angular: Vector3,
}
#[derive(Debug, Serialize, Deserialize)]
struct TwistStamped {
    header: Header,
    twist: Twist,
}
#[derive(Debug, Serialize, Deserialize)]
struct JointState {
    header: Header,
    name: Vec<String>,
    position: Vec<f64>,
    velocity: Vec<f64>,
    effort: Vec<f64>,
}
#[derive(Debug, Serialize, Deserialize)]
struct Log {
    stamp: Time,
    level: u8,
    name: String,
    msg: String,
    file: String,
    function: String,
    line: u32,
}

/// `0/kuka/pose/geometry_msgs::msg::dds_::PoseStamped_/RIHS01_...` -> `geometry_msgs/PoseStamped`
pub fn ros_type(key: &str) -> Option<String> {
    for seg in key.split('/') {
        if let Some(idx) = seg.find("::msg::dds_::") {
            let pkg = &seg[..idx];
            let rest = &seg[idx + "::msg::dds_::".len()..];
            let name = rest.strip_suffix('_').unwrap_or(rest);
            if !pkg.is_empty() && !name.is_empty() {
                return Some(format!("{pkg}/{name}"));
            }
        }
    }
    None
}

/// Readable key: drop the type tag + RIHS hash tail. `0/kuka/pose/<type>/<hash>` -> `0/kuka/pose`.
pub fn short_key(key: &str) -> String {
    let mut out = Vec::new();
    for seg in key.split('/') {
        if seg.contains("::msg::dds_::") || seg.starts_with("RIHS01_") {
            break;
        }
        out.push(seg);
    }
    if out.is_empty() {
        key.to_string()
    } else {
        out.join("/")
    }
}

fn decode_known(ros: &str, raw: &[u8]) -> Option<Value> {
    macro_rules! d {
        ($t:ty) => {
            cdr::deserialize::<$t>(raw)
                .ok()
                .and_then(|v| serde_json::to_value(v).ok())
        };
    }
    match ros {
        "std_msgs/String" => d!(StringMsg),
        "std_msgs/Bool" => d!(Bool),
        "std_msgs/Int32" => d!(Int32),
        "std_msgs/Int64" => d!(Int64),
        "std_msgs/Float32" => d!(Float32),
        "std_msgs/Float64" => d!(Float64),
        "geometry_msgs/PoseStamped" => d!(PoseStamped),
        "geometry_msgs/TwistStamped" => d!(TwistStamped),
        "geometry_msgs/Pose" => d!(Pose),
        "geometry_msgs/Twist" => d!(Twist),
        "sensor_msgs/JointState" => d!(JointState),
        "rcl_interfaces/Log" => d!(Log),
        _ => None,
    }
}

fn hex_preview(raw: &[u8]) -> String {
    let n = raw.len().min(48);
    let mut s = String::with_capacity(n * 2);
    for b in &raw[..n] {
        s.push_str(&format!("{b:02x}"));
    }
    if raw.len() > n {
        s.push('…');
    }
    s
}

/// Decode a payload to JSON given the (optional) ROS type inferred from its key.
/// Falls back to JSON/text, then to a hex preview for opaque binary.
pub fn decode_value(ros: Option<&str>, raw: &[u8]) -> Value {
    if let Some(rt) = ros {
        if let Some(v) = decode_known(rt, raw) {
            return v;
        }
        // Recognized ROS type tag but no struct/decoder for it (or decode failed).
        return json!({
            "_ros_type": rt,
            "_undecoded": true,
            "_bytes": raw.len(),
            "_hex": hex_preview(raw),
        });
    }
    if let Ok(s) = std::str::from_utf8(raw) {
        // Many non-ROS Zenoh payloads here are JSON strings — surface them structured.
        if let Ok(j) = serde_json::from_str::<Value>(s) {
            return j;
        }
        return json!(s);
    }
    json!({ "_binary": true, "_bytes": raw.len(), "_hex": hex_preview(raw) })
}
