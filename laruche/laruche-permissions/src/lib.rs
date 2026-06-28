use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    Default,
    Plan,
    AcceptEdits,
    Auto,
    Bubble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionBehavior {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RuleSource {
    UserSettings,
    ProjectSettings,
    LocalSettings,
    CliArg,
    Session,
    Policy,
    SwarmLeader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRule {
    pub source: RuleSource,
    pub behavior: PermissionBehavior,
    pub tool_name: String,
    pub rule_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionContext {
    pub mode: PermissionMode,
    pub rules: Vec<PermissionRule>,
    pub additional_working_directories: BTreeMap<PathBuf, RuleSource>,
    pub should_avoid_prompts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheck {
    pub tool_name: String,
    pub content: Option<String>,
    pub working_directory: Option<PathBuf>,
    pub is_write: bool,
    pub is_network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionUpdate {
    pub destination: RuleSource,
    pub behavior: PermissionBehavior,
    pub tool_name: String,
    pub rule_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDecision {
    pub behavior: PermissionBehavior,
    pub reason: String,
    pub suggestions: Vec<PermissionUpdate>,
}

impl Default for PermissionContext {
    fn default() -> Self {
        Self {
            mode: PermissionMode::Default,
            rules: Vec::new(),
            additional_working_directories: BTreeMap::new(),
            should_avoid_prompts: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PermissionEngine {
    context: PermissionContext,
}

impl PermissionEngine {
    pub fn new(context: PermissionContext) -> Self {
        Self { context }
    }

    pub fn context(&self) -> &PermissionContext {
        &self.context
    }

    pub fn apply_update(&mut self, update: PermissionUpdate) {
        self.context.rules.retain(|rule| {
            !(rule.source == update.destination
                && rule.behavior == update.behavior
                && rule.tool_name == update.tool_name
                && rule.rule_content == update.rule_content)
        });
        self.context.rules.push(PermissionRule {
            source: update.destination,
            behavior: update.behavior,
            tool_name: update.tool_name,
            rule_content: update.rule_content,
        });
    }

    pub fn add_working_directory(&mut self, path: impl Into<PathBuf>, source: RuleSource) {
        self.context
            .additional_working_directories
            .insert(path.into(), source);
    }

    pub fn rules_for_source(&self, source: RuleSource) -> Vec<&PermissionRule> {
        self.context
            .rules
            .iter()
            .filter(|rule| rule.source == source)
            .collect()
    }

    pub fn decide(&self, check: &PermissionCheck) -> PermissionDecision {
        if let Some(rule) = self.matching_rule(check, PermissionBehavior::Deny) {
            return PermissionDecision {
                behavior: PermissionBehavior::Deny,
                reason: format!("deny rule {:?} for {}", rule.source, rule.tool_name),
                suggestions: vec![],
            };
        }
        if let Some(rule) = self.matching_rule(check, PermissionBehavior::Allow) {
            return PermissionDecision {
                behavior: PermissionBehavior::Allow,
                reason: format!("allow rule {:?} for {}", rule.source, rule.tool_name),
                suggestions: vec![],
            };
        }

        match self.context.mode {
            PermissionMode::Plan if check.is_write => {
                PermissionDecision::deny("plan mode: writing forbidden")
            }
            PermissionMode::AcceptEdits if looks_like_edit_tool(&check.tool_name) => {
                PermissionDecision::allow("mode acceptEdits")
            }
            PermissionMode::Auto => PermissionDecision::allow("mode auto"),
            PermissionMode::Bubble => PermissionDecision::ask(
                "permission to escalate to the leader",
                self.suggest_allow(check, RuleSource::SwarmLeader),
            ),
            _ => {
                if self.context.should_avoid_prompts {
                    PermissionDecision::deny("prompts unavailable in this context")
                } else if self.path_allowed(check) && !check.is_network {
                    PermissionDecision::allow("working directory allowed")
                } else {
                    PermissionDecision::ask(
                        "no explicit rule",
                        self.suggest_allow(check, RuleSource::Session),
                    )
                }
            }
        }
    }

    pub fn assert_allowed(&self, check: &PermissionCheck) -> Result<()> {
        let decision = self.decide(check);
        match decision.behavior {
            PermissionBehavior::Allow => Ok(()),
            PermissionBehavior::Ask | PermissionBehavior::Deny => {
                bail!("permission denied: {}", decision.reason)
            }
        }
    }

    fn matching_rule(
        &self,
        check: &PermissionCheck,
        behavior: PermissionBehavior,
    ) -> Option<&PermissionRule> {
        self.context.rules.iter().find(|rule| {
            rule.behavior == behavior
                && tool_matches(&rule.tool_name, &check.tool_name)
                && rule
                    .rule_content
                    .as_ref()
                    .map(|content| {
                        check
                            .content
                            .as_ref()
                            .is_some_and(|value| value.contains(content))
                    })
                    .unwrap_or(true)
        })
    }

    fn path_allowed(&self, check: &PermissionCheck) -> bool {
        let Some(dir) = &check.working_directory else {
            return false;
        };
        self.context
            .additional_working_directories
            .keys()
            .any(|allowed| is_inside(dir, allowed))
    }

    fn suggest_allow(
        &self,
        check: &PermissionCheck,
        destination: RuleSource,
    ) -> Vec<PermissionUpdate> {
        vec![PermissionUpdate {
            destination,
            behavior: PermissionBehavior::Allow,
            tool_name: check.tool_name.clone(),
            rule_content: check.content.clone(),
        }]
    }
}

impl PermissionDecision {
    pub fn allow(reason: impl Into<String>) -> Self {
        Self {
            behavior: PermissionBehavior::Allow,
            reason: reason.into(),
            suggestions: vec![],
        }
    }

    pub fn ask(reason: impl Into<String>, suggestions: Vec<PermissionUpdate>) -> Self {
        Self {
            behavior: PermissionBehavior::Ask,
            reason: reason.into(),
            suggestions,
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            behavior: PermissionBehavior::Deny,
            reason: reason.into(),
            suggestions: vec![],
        }
    }
}

pub fn redact_sensitive_rules(rules: &[PermissionRule]) -> Vec<PermissionRule> {
    let sensitive: BTreeSet<&str> = [".git", ".ssh", ".env", ".assistant", ".laruche"]
        .into_iter()
        .collect();
    rules
        .iter()
        .filter(|rule| {
            !rule
                .rule_content
                .as_deref()
                .is_some_and(|content| sensitive.iter().any(|needle| content.contains(needle)))
        })
        .cloned()
        .collect()
}

fn looks_like_edit_tool(tool_name: &str) -> bool {
    tool_name.contains("edit") || tool_name.contains("write")
}

fn tool_matches(rule: &str, actual: &str) -> bool {
    rule == actual
        || rule == "*"
        || actual.starts_with(&format!("{rule}__"))
        || rule
            .strip_suffix('*')
            .is_some_and(|prefix| actual.starts_with(prefix))
}

fn is_inside(path: &Path, root: &Path) -> bool {
    let path = path.components().collect::<Vec<_>>();
    let root = root.components().collect::<Vec<_>>();
    path.starts_with(&root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(tool_name: &str) -> PermissionCheck {
        PermissionCheck {
            tool_name: tool_name.into(),
            content: None,
            working_directory: None,
            is_write: false,
            is_network: false,
        }
    }

    #[test]
    fn permission_engine_denies_before_allows_and_suggests_session_rule() {
        let mut ctx = PermissionContext {
            mode: PermissionMode::Default,
            ..PermissionContext::default()
        };
        ctx.rules.push(PermissionRule {
            source: RuleSource::Policy,
            behavior: PermissionBehavior::Allow,
            tool_name: "web_fetch".into(),
            rule_content: None,
        });
        ctx.rules.push(PermissionRule {
            source: RuleSource::Policy,
            behavior: PermissionBehavior::Deny,
            tool_name: "web_fetch".into(),
            rule_content: None,
        });
        let engine = PermissionEngine::new(ctx);
        let mut denied = check("web_fetch");
        denied.is_network = true;
        assert_eq!(engine.decide(&denied).behavior, PermissionBehavior::Deny);

        let asked = PermissionEngine::default().decide(&check("shell_exec"));
        assert_eq!(asked.behavior, PermissionBehavior::Ask);
        assert_eq!(asked.suggestions[0].destination, RuleSource::Session);
    }

    #[test]
    fn tool_matching_supports_exact_wildcard_and_mcp_prefix() {
        assert!(tool_matches("shell_exec", "shell_exec"));
        assert!(tool_matches("*", "anything"));
        assert!(tool_matches("mcp", "mcp__server__tool"));
        assert!(tool_matches("file_*", "file_read"));
        assert!(!tool_matches("web_fetch", "web_search"));
    }

    #[test]
    fn modes_have_expected_default_behaviors() {
        let mut write = check("file_write");
        write.is_write = true;
        let plan = PermissionEngine::new(PermissionContext {
            mode: PermissionMode::Plan,
            ..PermissionContext::default()
        });
        assert_eq!(plan.decide(&write).behavior, PermissionBehavior::Deny);

        let accept_edits = PermissionEngine::new(PermissionContext {
            mode: PermissionMode::AcceptEdits,
            ..PermissionContext::default()
        });
        assert_eq!(
            accept_edits.decide(&write).behavior,
            PermissionBehavior::Allow
        );

        let auto = PermissionEngine::new(PermissionContext {
            mode: PermissionMode::Auto,
            ..PermissionContext::default()
        });
        assert_eq!(
            auto.decide(&check("web_fetch")).behavior,
            PermissionBehavior::Allow
        );

        let bubble = PermissionEngine::new(PermissionContext {
            mode: PermissionMode::Bubble,
            ..PermissionContext::default()
        });
        let decision = bubble.decide(&check("shell_exec"));
        assert_eq!(decision.behavior, PermissionBehavior::Ask);
        assert_eq!(decision.suggestions[0].destination, RuleSource::SwarmLeader);
    }

    #[test]
    fn working_directory_and_rule_sources_are_preserved() {
        let mut engine = PermissionEngine::default();
        engine.add_working_directory("C:/work/project", RuleSource::ProjectSettings);
        engine.apply_update(PermissionUpdate {
            destination: RuleSource::UserSettings,
            behavior: PermissionBehavior::Allow,
            tool_name: "file_read".into(),
            rule_content: None,
        });

        let mut read = check("file_read");
        read.working_directory = Some(PathBuf::from("C:/work/project/src"));
        assert_eq!(engine.decide(&read).behavior, PermissionBehavior::Allow);
        assert_eq!(engine.rules_for_source(RuleSource::UserSettings).len(), 1);
    }
}
