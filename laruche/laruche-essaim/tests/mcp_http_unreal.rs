//! Live check of the HTTP MCP transport against a running Unreal Engine 5.8 editor.
//!
//! Ignored by default: it needs an editor open with the ModelContextProtocol and
//! AllToolsets plugins enabled, listening on http://127.0.0.1:8000/mcp. Run it with
//! `cargo test -p laruche-essaim --test mcp_http_unreal -- --ignored --nocapture`
//! after touching the transport, because the session handling it covers is exactly
//! what a unit test cannot reach.
use laruche_essaim::mcp_client::McpClient;

#[tokio::test]
#[ignore = "needs a live Unreal editor on port 8000"]
async fn le_serveur_unreal_expose_ses_outils() {
    let url = std::env::var("UNREAL_MCP_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8000/mcp".to_string());
    let client = McpClient::start_http(&url).await.expect("handshake");
    let outils = client.list_tools().await.expect("tools/list");
    let noms: Vec<&str> = outils.iter().map(|t| t.name.as_str()).collect();
    println!("outils exposes: {noms:?}");
    // bEnableToolSearch=True, so the server publishes its three meta tools and nothing
    // else. Zero tools is the symptom the session fix exists for.
    assert!(noms.contains(&"call_tool"), "call_tool missing, got {noms:?}");
    assert!(noms.contains(&"list_toolsets"), "list_toolsets missing, got {noms:?}");
}
