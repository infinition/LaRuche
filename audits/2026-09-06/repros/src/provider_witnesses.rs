//! Witnesses against the actual HTTP/SSE adapter, using a loopback-only fixture.
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn read_fixture(body: String, incomplete_http: bool) -> Vec<laruche_essaim::streaming::OllamaChunk> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let n = socket.read(&mut chunk).await.unwrap();
            assert!(n > 0);
            request.extend_from_slice(&chunk[..n]);
            if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]);
                let len = headers.lines().find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    k.eq_ignore_ascii_case("content-length").then(|| v.trim().parse::<usize>().unwrap())
                }).unwrap_or(0);
                if request.len() >= end + 4 + len { break; }
            }
        }
        let claimed_length = body.len() + if incomplete_http { 100 } else { 0 };
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {claimed_length}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        socket.write_all(body.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
    });
    let messages = vec![serde_json::json!({"role":"user", "content":"fixture"})];
    let stream = laruche_essaim::providers::provider_chat_stream(
        "openai", "audit-fixture", &messages, 0.0, 16, "", Some(&base), "", None,
    ).await.unwrap();
    let chunks = tokio::time::timeout(std::time::Duration::from_secs(5), stream.collect::<Vec<_>>()).await.unwrap();
    server.await.unwrap();
    chunks
}

#[tokio::test]
async fn witness_done_overwrites_explicit_length_with_stop() {
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"Super, l'app est\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let chunks = read_fixture(body.into(), false).await;
    let reasons: Vec<_> = chunks.iter().filter_map(|c| c.finish_reason.as_deref()).collect();
    assert_eq!(reasons, ["length", "stop"]);
    // FournisseurPont keeps the most recent non-None finish_reason.
    assert_eq!(reasons.last(), Some(&"stop"));
}

#[tokio::test]
async fn witness_reasoning_only_becomes_answer_and_normal_stop() {
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"I need to make a move, let me think\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let chunks = read_fixture(body.into(), false).await;
    assert!(chunks.iter().any(|c| c.text == "I need to make a move, let me think"));
    assert_eq!(chunks.iter().filter_map(|c| c.finish_reason.as_deref()).last(), Some("stop"));
}

#[tokio::test]
async fn witness_http_disconnect_delivers_text_without_error_or_terminal() {
    let body = "data: {\"choices\":[{\"delta\":{\"content\":\"unfinished\"},\"finish_reason\":null}]}\n\n";
    let chunks = read_fixture(body.into(), true).await;
    assert!(chunks.iter().any(|c| c.text == "unfinished"));
    assert!(chunks.iter().all(|c| c.finish_reason.is_none()));
}
