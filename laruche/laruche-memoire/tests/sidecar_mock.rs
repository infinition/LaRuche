//! Integration test: `SidecarBackend` against a fake paradigm bridge.
//!
//! We start a minimal HTTP server that mimics `paradigm serve` (endpoints `/health`
//! and `/mcp` JSON-RPC), then verify that the backend speaks the protocol correctly
//! end-to-end, without needing Node or a real paradigm installation.

use laruche_memoire::{MemoireCognitive, MemoryItem, SearchOpts, SidecarBackend, SidecarConfig};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Reads an HTTP request (headers + body) and returns (request line + headers, body).
async fn read_request(stream: &mut TcpStream) -> (String, String) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = stream.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find(&buf, b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..pos]).to_string();
            let content_length = head
                .lines()
                .find_map(|l| {
                    let l = l.to_ascii_lowercase();
                    l.strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            let body_start = pos + 4;
            while buf.len() - body_start < content_length {
                let n = stream.read(&mut tmp).await.unwrap();
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
            }
            let body = String::from_utf8_lossy(&buf[body_start..]).to_string();
            return (head, body);
        }
    }
    (String::new(), String::new())
}

async fn respond(stream: &mut TcpStream, body: &str) {
    let resp = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(resp.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
}

/// Wraps a result in the MCP `tools/call` shape that paradigm returns.
fn mcp_text_result(inner: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": { "content": [{ "type": "text", "text": inner.to_string() }] }
    })
    .to_string()
}

/// Starts the fake paradigm bridge, returns the base URL.
async fn start_fake_paradigm() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (head, body) = read_request(&mut stream).await;
            if head.starts_with("GET /health") {
                respond(
                    &mut stream,
                    &json!({ "ok": true, "name": "paradigm-memory" }).to_string(),
                )
                .await;
            } else if head.starts_with("POST /mcp") {
                if body.contains("memory_write") {
                    respond(
                        &mut stream,
                        &mcp_text_result(json!({ "ok": true, "item_id": "itm_42" })),
                    )
                    .await;
                } else {
                    // memory_search
                    let pack = json!({
                        "nodes": [{ "id": "decisions.archi", "one_liner": "choix d'architecture" }],
                        "items": [{ "content": "Cible = mono-binaire Rust. Sidecar paradigm = prototype." }]
                    });
                    respond(&mut stream, &mcp_text_result(pack)).await;
                }
            } else {
                respond(&mut stream, &json!({ "error": "not_found" }).to_string()).await;
            }
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn sidecar_parle_le_protocole_paradigm() {
    let base_url = start_fake_paradigm().await;
    let backend = SidecarBackend::new(SidecarConfig {
        base_url,
        workspace: Some("laruche".into()),
        token: None,
    });

    // health
    assert!(
        backend.health().await.unwrap(),
        "the sidecar should be healthy"
    );

    // search: the rendered context pack must contain the evidence item content
    let pack = backend
        .search("architecture", SearchOpts::default())
        .await
        .unwrap();
    let text = pack.to_prompt_text();
    assert!(
        text.contains("mono-binaire Rust"),
        "rendered context pack = {text}"
    );
    assert!(
        text.contains("decisions.archi"),
        "the activated node must appear"
    );

    // write: we retrieve the item id returned by the engine
    let res = backend
        .write(
            MemoryItem::new("decisions.archi", "On garde SQLite comme source de vérité.")
                .with_source("conversation"),
        )
        .await
        .unwrap();
    assert_eq!(res.get("item_id").and_then(Value::as_str), Some("itm_42"));
}
