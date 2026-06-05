//! zenoh-mcp — a stdio MCP server exposing Zenoh debugging tools for agents.
//!
//! Pinned to zenoh 1.8.0 so it is wire-compatible with a 1.8.0 router and with
//! ROS2 Kilted's rmw_zenoh. Each tool opens a short-lived **client** session to
//! the target router, performs one operation, and closes — stateless and safe
//! to point at any router on the network.

mod decode;

use std::time::{Duration, Instant};

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::{tool, tool_handler, tool_router, schemars, ErrorData as McpError, ServerHandler};
use rmcp::{transport::stdio, ServiceExt};
use serde::Deserialize;
use serde_json::{json, Value};
use zenoh::Config;

const DEFAULT_CONNECT: &str = "tcp/localhost:7447";

fn resolve_connect(opt: Option<String>) -> String {
    opt.filter(|s| !s.is_empty())
        .or_else(|| std::env::var("ZENOH_CONNECT").ok())
        .unwrap_or_else(|| DEFAULT_CONNECT.to_string())
}

fn mcp_err(e: impl std::fmt::Display) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

fn ok_json(v: Value) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

/// Open a short-lived client-mode session connected to `connect`.
async fn open_session(connect: &str) -> Result<zenoh::Session, McpError> {
    let mut config = Config::default();
    config
        .insert_json5("mode", "\"client\"")
        .map_err(mcp_err)?;
    config
        .insert_json5("connect/endpoints", &format!("[\"{connect}\"]"))
        .map_err(mcp_err)?;
    // Don't wait on multicast scouting — we connect to an explicit router.
    config
        .insert_json5("scouting/multicast/enabled", "false")
        .map_err(mcp_err)?;
    zenoh::open(config).await.map_err(mcp_err)
}

/// Build a structured record for one sample/reply.
fn sample_record(key: &str, kind: &str, raw: &[u8]) -> Value {
    let rt = decode::ros_type(key);
    json!({
        "key": decode::short_key(key),
        "ros_type": rt,
        "kind": kind,
        "value": decode::decode_value(rt.as_deref(), raw),
    })
}

// ---- tool argument structs ----

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct InfoArgs {
    /// Router endpoint, e.g. "tcp/localhost:7447". Defaults to $ZENOH_CONNECT
    /// or tcp/localhost:7447.
    #[serde(default)]
    connect: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetArgs {
    /// Selector / key expression to query, e.g. "@/**" for router admin or
    /// "demo/**" for data. Supports wildcards.
    selector: String,
    /// Router endpoint (default $ZENOH_CONNECT or tcp/localhost:7447).
    #[serde(default)]
    connect: Option<String>,
    /// Max time to wait for replies, milliseconds (default 3000, max 30000).
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PutArgs {
    /// Key expression to publish to, e.g. "demo/test".
    key: String,
    /// Value to publish (sent as a UTF-8 payload).
    value: String,
    /// Router endpoint (default $ZENOH_CONNECT or tcp/localhost:7447).
    #[serde(default)]
    connect: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SubscribeArgs {
    /// Key expression to subscribe to (default "**" = everything).
    #[serde(default)]
    key: Option<String>,
    /// Router endpoint (default $ZENOH_CONNECT or tcp/localhost:7447).
    #[serde(default)]
    connect: Option<String>,
    /// How long to collect samples, seconds (default 3, max 30).
    #[serde(default)]
    duration_secs: Option<u64>,
    /// Stop early after this many samples (default 100, max 2000).
    #[serde(default)]
    max_samples: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ListKeysArgs {
    /// Key expression to sweep (default "**").
    #[serde(default)]
    key: Option<String>,
    /// Router endpoint (default $ZENOH_CONNECT or tcp/localhost:7447).
    #[serde(default)]
    connect: Option<String>,
    /// How long to listen before reporting, seconds (default 3, max 30).
    #[serde(default)]
    duration_secs: Option<u64>,
}

#[derive(Clone)]
struct ZenohMcp {
    // Read by the #[tool_handler] macro expansion, not by hand-written code.
    #[allow(dead_code)]
    tool_router: ToolRouter<ZenohMcp>,
}

#[tool_router]
impl ZenohMcp {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Connect to a Zenoh router and report session info (our Zenoh ID, the endpoint, and connected transports). Use this to confirm reachability before other calls."
    )]
    async fn zenoh_info(
        &self,
        Parameters(args): Parameters<InfoArgs>,
    ) -> Result<CallToolResult, McpError> {
        let connect = resolve_connect(args.connect);
        let session = open_session(&connect).await?;
        let zid = session.zid().to_string();
        // Query the router admin space for the peer we're attached to.
        let mut routers = Vec::new();
        if let Ok(replies) = session.get("@/**/router").await {
            while let Ok(Ok(r)) =
                tokio::time::timeout(Duration::from_millis(1500), replies.recv_async()).await
            {
                if let Ok(sample) = r.result() {
                    routers.push(sample.key_expr().as_str().to_string());
                }
            }
        }
        session.close().await.ok();
        ok_json(json!({
            "connected": true,
            "endpoint": connect,
            "session_zid": zid,
            "zenoh_version": "1.8.0",
            "router_admin_keys": routers,
        }))
    }

    #[tool(
        description = "Send a Zenoh GET (query) for a selector and return replies. Works for data queryables and for router admin info (e.g. selector \"@/**\" for topology, plugins, transports). ROS2 CDR replies are decoded to JSON."
    )]
    async fn zenoh_get(
        &self,
        Parameters(args): Parameters<GetArgs>,
    ) -> Result<CallToolResult, McpError> {
        let connect = resolve_connect(args.connect);
        let timeout = Duration::from_millis(args.timeout_ms.unwrap_or(3000).min(30_000));
        let session = open_session(&connect).await?;
        let replies = session.get(&args.selector).await.map_err(mcp_err)?;

        let mut out = Vec::new();
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, replies.recv_async()).await {
                Ok(Ok(reply)) => match reply.result() {
                    Ok(sample) => {
                        let key = sample.key_expr().as_str().to_string();
                        let raw = sample.payload().to_bytes();
                        out.push(sample_record(&key, &format!("{:?}", sample.kind()), &raw));
                    }
                    Err(e) => {
                        let body = e.payload().try_to_string().map(|c| c.into_owned());
                        out.push(json!({ "error": body.unwrap_or_else(|_| "<binary error>".into()) }));
                    }
                },
                Ok(Err(_)) => break, // channel closed: all replies received
                Err(_) => break,     // timed out
            }
        }
        session.close().await.ok();
        ok_json(json!({ "selector": args.selector, "endpoint": connect, "count": out.len(), "replies": out }))
    }

    #[tool(
        description = "Publish (PUT) a UTF-8 value to a Zenoh key expression. Useful for poking queryables/subscribers during debugging."
    )]
    async fn zenoh_put(
        &self,
        Parameters(args): Parameters<PutArgs>,
    ) -> Result<CallToolResult, McpError> {
        let connect = resolve_connect(args.connect);
        let session = open_session(&connect).await?;
        session
            .put(&args.key, args.value.clone())
            .await
            .map_err(mcp_err)?;
        session.close().await.ok();
        ok_json(json!({ "ok": true, "key": args.key, "bytes": args.value.len(), "endpoint": connect }))
    }

    #[tool(
        description = "Subscribe to a key expression for a few seconds and return the samples received, with ROS2 CDR payloads decoded to JSON. The primary tool for 'what is being published on this topic right now?'."
    )]
    async fn zenoh_subscribe(
        &self,
        Parameters(args): Parameters<SubscribeArgs>,
    ) -> Result<CallToolResult, McpError> {
        let connect = resolve_connect(args.connect);
        let key = args.key.unwrap_or_else(|| "**".to_string());
        let dur = Duration::from_secs(args.duration_secs.unwrap_or(3).clamp(1, 30));
        let max = args.max_samples.unwrap_or(100).clamp(1, 2000);

        let session = open_session(&connect).await?;
        let sub = session.declare_subscriber(&key).await.map_err(mcp_err)?;

        let mut out = Vec::new();
        let deadline = Instant::now() + dur;
        loop {
            if out.len() >= max {
                break;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, sub.recv_async()).await {
                Ok(Ok(sample)) => {
                    let k = sample.key_expr().as_str().to_string();
                    let raw = sample.payload().to_bytes();
                    out.push(sample_record(&k, &format!("{:?}", sample.kind()), &raw));
                }
                Ok(Err(_)) => break,
                Err(_) => break,
            }
        }
        session.close().await.ok();
        ok_json(json!({
            "key": key,
            "endpoint": connect,
            "duration_secs": dur.as_secs(),
            "count": out.len(),
            "truncated": out.len() >= max,
            "samples": out,
        }))
    }

    #[tool(
        description = "Discover topics: listen briefly on a key expression and return the distinct keys seen, each with its inferred ROS2 type and a sample count. Like 'ros2 topic list' for the Zenoh network."
    )]
    async fn zenoh_list_keys(
        &self,
        Parameters(args): Parameters<ListKeysArgs>,
    ) -> Result<CallToolResult, McpError> {
        let connect = resolve_connect(args.connect);
        let key = args.key.unwrap_or_else(|| "**".to_string());
        let dur = Duration::from_secs(args.duration_secs.unwrap_or(3).clamp(1, 30));

        let session = open_session(&connect).await?;
        let sub = session.declare_subscriber(&key).await.map_err(mcp_err)?;

        use std::collections::BTreeMap;
        let mut seen: BTreeMap<String, (u64, Option<String>)> = BTreeMap::new();
        let deadline = Instant::now() + dur;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, sub.recv_async()).await {
                Ok(Ok(sample)) => {
                    let k = sample.key_expr().as_str().to_string();
                    let rt = decode::ros_type(&k);
                    let entry = seen.entry(decode::short_key(&k)).or_insert((0, rt));
                    entry.0 += 1;
                }
                Ok(Err(_)) => break,
                Err(_) => break,
            }
        }
        session.close().await.ok();

        let keys: Vec<Value> = seen
            .into_iter()
            .map(|(k, (count, rt))| json!({ "key": k, "ros_type": rt, "samples": count }))
            .collect();
        ok_json(json!({
            "endpoint": connect,
            "duration_secs": dur.as_secs(),
            "distinct_keys": keys.len(),
            "keys": keys,
        }))
    }
}

#[tool_handler]
impl ServerHandler for ZenohMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder().enable_tools().build(),
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions(
            "Zenoh debugging tools (zenoh 1.8.0, ROS2-compatible). Tools: zenoh_info \
             (check reachability), zenoh_get (query a selector / router admin @/**), \
             zenoh_put (publish a value), zenoh_subscribe (tap a topic for N seconds, \
             ROS2 CDR decoded), zenoh_list_keys (discover topics). Set the router with \
             each tool's `connect` arg or the ZENOH_CONNECT env var."
                .to_string(),
        )
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "zenoh_mcp=info,zenoh=warn".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!(
        "starting zenoh-mcp (zenoh 1.8.0); default router {}",
        resolve_connect(None)
    );

    let service = ZenohMcp::new().serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serve error: {e:?}");
    })?;
    service.waiting().await?;
    Ok(())
}
