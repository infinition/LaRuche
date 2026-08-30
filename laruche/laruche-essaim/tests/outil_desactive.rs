use laruche_essaim::{AbeilleRegistry, ContextExecution};

/// `disabled_tools` used to be a DISPLAY filter only: it removed the tool from the schema
/// shown to the model, and nothing more. An agent that knew the name anyway - through
/// `tool_call`, `tool_search`, or its own memory of an earlier turn - still ran it, and so
/// did every MCP client. The guard now sits at the single point every execution goes
/// through, so "off" means off whoever is asking.
#[tokio::test]
async fn un_outil_desactive_est_refuse_a_lexecution() {
    // A registry with one real tool is enough: what is under test is the guard, not
    // the tool. `mission_list` need not even exist for a refusal to be correct.
    let registre = AbeilleRegistry::new();
    let mut ctx = ContextExecution::default();
    ctx.disabled_tools = vec!["mission_list".to_string()];

    let r = registre
        .executer("mission_list", serde_json::json!({}), &ctx)
        .await
        .expect("l'appel doit repondre, pas exploser");
    assert!(!r.success, "l'outil desactive s'est execute quand meme");
    let msg = r.error.unwrap_or(r.output);
    assert!(msg.contains("disabled"), "message peu clair: {msg}");

    // Sans liste de desactives, le garde laisse passer : l'appel echoue plus loin
    // (outil inconnu de ce registre nu), avec un message DIFFERENT de "disabled".
    let sans = registre
        .executer(
            "mission_list",
            serde_json::json!({}),
            &ContextExecution::default(),
        )
        .await
        .expect("appel sans liste de desactives");
    let m2 = sans.error.unwrap_or(sans.output);
    assert!(
        !m2.contains("disabled"),
        "le garde bloque alors que rien n'est desactive: {m2}"
    );
}
