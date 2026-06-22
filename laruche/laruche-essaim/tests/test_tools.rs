//! Integration tests for Essaim tools and utilities.

use laruche_essaim::abeille::{Abeille, ContextExecution, NiveauDanger};
use laruche_essaim::abeilles::execute_code::ExecuteCode;
use laruche_essaim::abeilles::fichiers::{FileList, FileRead, FileWrite};
use laruche_essaim::abeilles::math::MathEval;
use laruche_essaim::abeilles::read_extract::ReadExtract;
use laruche_essaim::abeilles::todo::Todo;
use laruche_essaim::brain::{
    parse_plan, parse_tool_calls, schema_outils_pour_prompt, EssaimConfig,
};
use laruche_essaim::session::Session;

fn default_ctx() -> ContextExecution {
    ContextExecution::default()
}

// ======================== ExecuteCode / Todo Tests ========================

#[tokio::test]
async fn test_execute_code_python_snippet() {
    if which::which("python").is_err() && which::which("python3").is_err() {
        return;
    }

    let tool = ExecuteCode;
    assert_eq!(tool.niveau_danger(), NiveauDanger::NeedsApproval);
    let result = tool
        .executer(
            serde_json::json!({"code": "print('laruche', 2 + 3)"}),
            &default_ctx(),
        )
        .await
        .unwrap();
    assert!(result.success, "execute_code failed: {:?}", result.error);
    assert!(result.output.contains("laruche 5"));
}

#[tokio::test]
async fn test_todo_add_list_done() {
    let tool = Todo;
    assert_eq!(tool.niveau_danger(), NiveauDanger::Safe);
    let text = format!(
        "tester todo {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let added = tool
        .executer(
            serde_json::json!({"action": "add", "text": text}),
            &default_ctx(),
        )
        .await
        .unwrap();
    assert!(added.success);
    let id: u64 = added
        .output
        .split('#')
        .nth(1)
        .and_then(|s| s.split(':').next())
        .unwrap()
        .parse()
        .unwrap();

    let listed = tool
        .executer(serde_json::json!({"action": "list"}), &default_ctx())
        .await
        .unwrap();
    assert!(listed.success);
    assert!(listed.output.contains(&format!("#{id}")));

    let done = tool
        .executer(
            serde_json::json!({"action": "done", "id": id}),
            &default_ctx(),
        )
        .await
        .unwrap();
    assert!(done.success);

    let listed = tool
        .executer(serde_json::json!({"action": "list"}), &default_ctx())
        .await
        .unwrap();
    assert!(listed.output.contains(&format!("[x] #{id}")));
}

// ======================== MathEval Tests ========================

#[tokio::test]
async fn test_math_eval_basic_addition() {
    let tool = MathEval;
    let args = serde_json::json!({"expression": "2 + 3"});
    let result = tool.executer(args, &default_ctx()).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("5"));
}

#[tokio::test]
async fn test_math_eval_multiplication() {
    let tool = MathEval;
    let args = serde_json::json!({"expression": "42 * 3.14"});
    let result = tool.executer(args, &default_ctx()).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("131.88"));
}

#[tokio::test]
async fn test_math_eval_sqrt() {
    let tool = MathEval;
    let args = serde_json::json!({"expression": "sqrt(16)"});
    let result = tool.executer(args, &default_ctx()).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("4"));
}

#[tokio::test]
async fn test_math_eval_power() {
    let tool = MathEval;
    let args = serde_json::json!({"expression": "2 ^ 10"});
    let result = tool.executer(args, &default_ctx()).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("1024"));
}

#[tokio::test]
async fn test_math_eval_division_by_zero() {
    let tool = MathEval;
    let args = serde_json::json!({"expression": "1 / 0"});
    let result = tool.executer(args, &default_ctx()).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Division by zero"));
}

#[tokio::test]
async fn test_math_eval_pi_constant() {
    let tool = MathEval;
    let args = serde_json::json!({"expression": "pi"});
    let result = tool.executer(args, &default_ctx()).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("3.14"));
}

#[tokio::test]
async fn test_math_eval_complex_expression() {
    let tool = MathEval;
    let args = serde_json::json!({"expression": "(42 * 3.14) + sqrt(16)"});
    let result = tool.executer(args, &default_ctx()).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("135.88"));
}

#[tokio::test]
async fn test_math_eval_missing_arg() {
    let tool = MathEval;
    let args = serde_json::json!({});
    let result = tool.executer(args, &default_ctx()).await;
    assert!(result.is_err());
}

// ======================== File Read/Write Tests ========================

#[tokio::test]
async fn test_file_write_and_read() {
    let dir = std::env::temp_dir().join("laruche_test_rw");
    let _ = std::fs::create_dir_all(&dir);
    let file_path = dir.join("test_write.txt");
    let path_str = file_path.to_string_lossy().to_string();

    // Write
    let write_tool = FileWrite;
    let args = serde_json::json!({"path": path_str, "content": "Hello, LaRuche!"});
    let result = write_tool.executer(args, &default_ctx()).await.unwrap();
    assert!(result.success, "FileWrite failed: {:?}", result.error);

    // Read
    let read_tool = FileRead;
    let args = serde_json::json!({"path": path_str});
    let result = read_tool.executer(args, &default_ctx()).await.unwrap();
    assert!(result.success, "FileRead failed: {:?}", result.error);
    // file_read renvoie désormais le contenu avec numéros de ligne (boost third-party-style).
    assert!(
        result.output.contains("Hello, LaRuche!"),
        "FileRead output = {:?}",
        result.output
    );

    // Cleanup
    let _ = std::fs::remove_file(&file_path);
    let _ = std::fs::remove_dir(&dir);
}

#[tokio::test]
async fn test_file_read_nonexistent() {
    let read_tool = FileRead;
    let args = serde_json::json!({"path": "/tmp/laruche_test_nonexistent_file_xyz.txt"});
    let result = read_tool.executer(args, &default_ctx()).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("not found"));
}

#[tokio::test]
async fn test_read_extract_txt_head_tail() {
    let dir = std::env::temp_dir().join("laruche_test_read_extract");
    let _ = std::fs::create_dir_all(&dir);
    let file_path = dir.join("long.md");
    let content = format!(
        "DEBUT\n{}\nFIN",
        (0..14000)
            .map(|i| if i % 2 == 0 { 'a' } else { 'b' })
            .collect::<String>()
    );
    std::fs::write(&file_path, content).unwrap();

    let tool = ReadExtract;
    assert_eq!(tool.niveau_danger(), NiveauDanger::Safe);
    let result = tool
        .executer(
            serde_json::json!({"path": file_path.to_string_lossy()}),
            &default_ctx(),
        )
        .await
        .unwrap();
    assert!(result.success, "read_extract failed: {:?}", result.error);
    assert!(result.output.contains("DEBUT"));
    assert!(result.output.contains("FIN"));
    assert!(result.output.contains("milieu tronque"));

    let _ = std::fs::remove_file(file_path);
    let _ = std::fs::remove_dir(&dir);
}

#[tokio::test]
async fn test_file_list() {
    let dir = std::env::temp_dir().join("laruche_test_list");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("alpha.txt"), "a");
    let _ = std::fs::write(dir.join("beta.txt"), "b");

    let tool = FileList;
    let args = serde_json::json!({"path": dir.to_string_lossy()});
    let result = tool.executer(args, &default_ctx()).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("alpha.txt"));
    assert!(result.output.contains("beta.txt"));

    // Cleanup
    let _ = std::fs::remove_file(dir.join("alpha.txt"));
    let _ = std::fs::remove_file(dir.join("beta.txt"));
    let _ = std::fs::remove_dir(&dir);
}

#[tokio::test]
async fn test_file_write_creates_parent_dirs() {
    let dir = std::env::temp_dir().join("laruche_test_nested/subdir");
    let file_path = dir.join("nested_file.txt");
    let path_str = file_path.to_string_lossy().to_string();

    let tool = FileWrite;
    let args = serde_json::json!({"path": path_str, "content": "nested content"});
    let result = tool.executer(args, &default_ctx()).await.unwrap();
    assert!(
        result.success,
        "FileWrite nested failed: {:?}",
        result.error
    );
    assert!(file_path.exists());

    // Cleanup
    let _ = std::fs::remove_file(&file_path);
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("laruche_test_nested"));
}

// ======================== parse_tool_calls Tests ========================

#[test]
fn test_parse_tool_calls_single() {
    let text = r#"I'll read that file for you.
<tool_call>{"name": "file_read", "arguments": {"path": "/tmp/test.txt"}}</tool_call>"#;

    let calls = parse_tool_calls(text);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "file_read");
    assert_eq!(calls[0].args["path"], "/tmp/test.txt");
}

#[test]
fn test_parse_tool_calls_multiple() {
    let text = r#"Let me do two things.
<tool_call>{"name": "file_read", "arguments": {"path": "a.txt"}}</tool_call>
Then also:
<tool_call>{"name": "math_eval", "arguments": {"expression": "2+2"}}</tool_call>"#;

    let calls = parse_tool_calls(text);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "file_read");
    assert_eq!(calls[1].name, "math_eval");
}

#[test]
fn test_parse_tool_calls_none() {
    let text = "Just a regular response with no tool calls.";
    let calls = parse_tool_calls(text);
    assert!(calls.is_empty());
}

#[test]
fn test_parse_tool_calls_malformed_json() {
    let text = r#"<tool_call>not valid json</tool_call>"#;
    let calls = parse_tool_calls(text);
    assert!(calls.is_empty());
}

#[test]
fn test_parse_tool_calls_unclosed_tag() {
    let text = r#"<tool_call>{"name": "test", "arguments": {}}"#;
    let calls = parse_tool_calls(text);
    assert!(calls.is_empty());
}

// ======================== parse_plan Tests ========================

#[test]
fn test_parse_plan_valid() {
    let text = r#"Here's my plan:
<plan>[{"task": "Read the file", "status": "done"}, {"task": "Analyze contents", "status": "pending"}]</plan>
Let me start."#;

    let plan = parse_plan(text);
    assert!(plan.is_some());
    let items = plan.unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].task, "Read the file");
    assert_eq!(items[0].status, "done");
    assert_eq!(items[1].task, "Analyze contents");
    assert_eq!(items[1].status, "pending");
}

#[test]
fn test_parse_plan_none() {
    let text = "No plan in this response.";
    let plan = parse_plan(text);
    assert!(plan.is_none());
}

#[test]
fn test_parse_plan_malformed() {
    let text = "<plan>not valid json</plan>";
    let plan = parse_plan(text);
    assert!(plan.is_none());
}

// ======================== Session Tests ========================

#[test]
fn test_session_create() {
    let session = Session::new("test-model");
    assert!(session.messages.is_empty());
    assert_eq!(session.model, "test-model");
    assert!(session.title.is_none());
}

#[test]
fn test_session_add_messages() {
    let mut session = Session::new("test-model");
    session.ajouter_user("Hello");
    session.ajouter_assistant("Hi there!");
    session.ajouter_observation("file_read", "file contents here");

    assert_eq!(session.messages.len(), 3);
}

#[test]
fn test_session_save_and_load() {
    let dir = std::env::temp_dir().join("laruche_test_sessions");
    let _ = std::fs::create_dir_all(&dir);

    let mut session = Session::new_with_path("test-model", &dir);
    session.ajouter_user("What is Rust?");
    session.ajouter_assistant("Rust is a systems programming language.");
    session.sauvegarder().unwrap();

    // Find the saved file
    let session_file = dir.join(format!("{}.json", session.id));
    assert!(session_file.exists(), "Session file should exist");

    // Load it back
    let loaded = Session::charger(&session_file).unwrap();
    assert_eq!(loaded.id, session.id);
    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(loaded.model, "test-model");

    // Cleanup
    let _ = std::fs::remove_file(&session_file);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn test_session_build_ollama_messages() {
    let mut session = Session::new("test-model");
    session.ajouter_user("Hello");
    session.ajouter_assistant("Hi!");

    let msgs = session.build_ollama_messages("You are a helpful assistant.");
    // Should have: system + user + assistant = 3 messages
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0]["role"], "system");
    assert_eq!(msgs[1]["role"], "user");
    assert_eq!(msgs[2]["role"], "assistant");
}

#[test]
fn test_session_compacter_uses_structured_summary_and_tool_result_store() {
    let dir = std::env::temp_dir().join("laruche_test_compaction");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mut session = Session::new_with_path("test-model", &dir);
    session.ajouter_user("Construis un plan de refactor");
    session.ajouter_assistant("Je vais inspecter le code et proposer une sequence.");
    session.ajouter_observation("read_extract", &"x".repeat(5_000));
    session.ajouter_user("Continue");
    session.ajouter_assistant("Suite recente");
    session.compacter(2);

    assert_eq!(session.messages.len(), 3);
    match &session.messages[0] {
        laruche_essaim::session::Message::System(text) => {
            assert!(text.contains("Resume structure"));
            assert!(text.contains("Demande principale"));
        }
        other => panic!("expected summary system message, got {other:?}"),
    }
    let store_dir = dir.join("tool-results").join(session.id.to_string());
    assert!(store_dir.exists());
    assert!(std::fs::read_dir(&store_dir).unwrap().next().is_some());

    let _ = std::fs::remove_dir_all(&dir);
}

// ======================== AbeilleRegistry Tests ========================

#[test]
fn test_registry_creation_and_tool_list() {
    let mut registry = laruche_essaim::AbeilleRegistry::new();
    laruche_essaim::abeilles::enregistrer_abeilles_builtin(&mut registry);

    let names = registry.noms();
    assert!(
        names.len() >= 10,
        "Should have at least 10 built-in tools, got {}",
        names.len()
    );
    for expected in [
        "file_read",
        "read_extract",
        "file_write",
        "math_eval",
        "shell_exec",
        "web_search",
        "media_present",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing built-in tool: {expected}"
        );
    }
}

#[test]
fn test_registry_schema_complet() {
    let mut registry = laruche_essaim::AbeilleRegistry::new();
    laruche_essaim::abeilles::enregistrer_abeilles_builtin(&mut registry);

    let schema = registry.schema_complet();
    let tools = schema
        .as_array()
        .expect("schema_complet should return array");
    assert!(!tools.is_empty());

    // Each tool should have name, description, parameters
    for tool in tools {
        assert!(tool["name"].is_string(), "Tool missing name");
        assert!(tool["description"].is_string(), "Tool missing description");
        assert!(tool["parameters"].is_object(), "Tool missing parameters");
    }
}

#[test]
fn test_dynamic_tool_selection_keeps_core_and_adds_relevant_tools() {
    let mut registry = laruche_essaim::AbeilleRegistry::new();
    laruche_essaim::abeilles::enregistrer_abeilles_builtin(&mut registry);
    let full_len = registry.schema_complet().as_array().unwrap().len();

    let mut cfg = EssaimConfig::default();
    cfg.dynamic_tool_selection = true;
    cfg.tool_selection_limit = 9;

    let schema = schema_outils_pour_prompt(
        &registry,
        &cfg,
        "montre le git diff puis explique le status du depot",
    );
    let tools = schema.as_array().unwrap();
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    assert!(tools.len() < full_len);
    assert!(names.contains(&"file_read"));
    assert!(names.contains(&"shell_exec"));
    assert!(names.contains(&"git_diff"));
}

#[test]
fn test_dynamic_tool_selection_respects_disabled_tools() {
    let mut registry = laruche_essaim::AbeilleRegistry::new();
    laruche_essaim::abeilles::enregistrer_abeilles_builtin(&mut registry);

    let mut cfg = EssaimConfig::default();
    cfg.dynamic_tool_selection = true;
    cfg.tool_selection_limit = 12;
    cfg.disabled_tools = vec!["shell_exec".to_string()];

    let schema = schema_outils_pour_prompt(&registry, &cfg, "execute une commande shell");
    let names: Vec<&str> = schema
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();

    assert!(!names.contains(&"shell_exec"));
}

#[tokio::test]
async fn test_registry_execute_unknown_tool() {
    let mut registry = laruche_essaim::AbeilleRegistry::new();
    laruche_essaim::abeilles::enregistrer_abeilles_builtin(&mut registry);

    let result = registry
        .executer("nonexistent_tool", serde_json::json!({}), &default_ctx())
        .await
        .unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Unknown tool"));
}
