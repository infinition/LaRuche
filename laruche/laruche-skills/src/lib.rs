//! OKF skills for LaRuche.
//!
//! A skill is a Markdown document with YAML frontmatter (`type: skill`) and a body
//! structured into paradigms and steps. Storage lives in the cognitive memory via
//! `capacities.skills.<name>`; this crate creates no store.

use anyhow::{anyhow, Result};
use laruche_memoire::{MemoireCognitive, MemoryItem, SearchOpts};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const SKILLS_ROOT_NODE: &str = "capacities.skills";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillMeta {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "allowed-tools", default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub when_to_use: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Paradigm {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Step {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub instruction: String,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    #[serde(default)]
    pub execution: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub human_checkpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Skill {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub meta: SkillMeta,
    #[serde(default)]
    pub paradigms: Vec<Paradigm>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub custom_attributes: BTreeMap<String, String>,
}

impl Skill {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            kind: "skill".to_string(),
            meta: SkillMeta {
                name: name.into(),
                description: description.into(),
                allowed_tools: Vec::new(),
                when_to_use: String::new(),
                arguments: Vec::new(),
                context: None,
            },
            paradigms: Vec::new(),
            steps: Vec::new(),
            custom_attributes: BTreeMap::new(),
        }
    }

    pub fn parse(markdown: &str) -> Result<Self> {
        let (frontmatter, body) = split_frontmatter(markdown)?;
        let fields = parse_frontmatter(frontmatter)?;
        let kind = fields
            .get("type")
            .cloned()
            .unwrap_or_else(|| "skill".to_string());
        if kind != "skill" {
            return Err(anyhow!("invalid OKF type for a skill: {kind}"));
        }

        let meta = SkillMeta {
            name: required(&fields, "name")?,
            description: fields.get("description").cloned().unwrap_or_default(),
            allowed_tools: parse_array(fields.get("allowed-tools")),
            when_to_use: fields.get("when_to_use").cloned().unwrap_or_default(),
            arguments: parse_array(fields.get("arguments")),
            context: fields.get("context").cloned(),
        };
        let (paradigms, steps) = parse_body(body);
        Ok(Self {
            kind,
            meta,
            paradigms,
            steps,
            custom_attributes: fields
                .into_iter()
                .filter(|(k, _)| {
                    !matches!(
                        k.as_str(),
                        "type"
                            | "name"
                            | "description"
                            | "allowed-tools"
                            | "when_to_use"
                            | "arguments"
                            | "context"
                    )
                })
                .collect(),
        })
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("---\n");
        out.push_str("type: skill\n");
        out.push_str(&format!("name: {}\n", yaml_scalar(&self.meta.name)));
        out.push_str(&format!(
            "description: {}\n",
            yaml_scalar(&self.meta.description)
        ));
        out.push_str(&format!(
            "allowed-tools: {}\n",
            yaml_array(&self.meta.allowed_tools)
        ));
        out.push_str(&format!(
            "when_to_use: {}\n",
            yaml_scalar(&self.meta.when_to_use)
        ));
        if !self.meta.arguments.is_empty() {
            out.push_str(&format!(
                "arguments: {}\n",
                yaml_array(&self.meta.arguments)
            ));
        }
        if let Some(context) = &self.meta.context {
            out.push_str(&format!("context: {}\n", yaml_scalar(context)));
        }
        for (k, v) in &self.custom_attributes {
            out.push_str(&format!("{k}: {}\n", yaml_scalar(v)));
        }
        out.push_str("---\n\n");
        out.push_str(&format!("# Skill: {}\n\n", self.meta.name));

        if !self.paradigms.is_empty() {
            out.push_str("# Principes et Paradigmes\n\n");
            for p in &self.paradigms {
                out.push_str(&format!("## Paradigm: {}\n", p.title));
                if !p.description.trim().is_empty() {
                    out.push_str(&format!("{}\n\n", p.description.trim()));
                }
                for rule in &p.rules {
                    out.push_str(&format!("- {}\n", rule));
                }
                out.push('\n');
            }
        }

        if !self.steps.is_empty() {
            out.push_str("# Processus et Etapes de Competence\n\n");
            for s in &self.steps {
                out.push_str(&format!("## Step: {}\n", s.name));
                if !s.instruction.trim().is_empty() {
                    out.push_str(&format!("{}\n\n", s.instruction.trim()));
                }
                if let Some(execution) = &s.execution {
                    out.push_str(&format!("- Execution: {}\n", execution));
                }
                if !s.artifacts.is_empty() {
                    out.push_str(&format!("- Artifacts: {}\n", s.artifacts.join(", ")));
                }
                if s.human_checkpoint {
                    out.push_str("- Human checkpoint: true\n");
                }
                for criterion in &s.success_criteria {
                    out.push_str(&format!("- Criterion: {}\n", criterion));
                }
                out.push('\n');
            }
        }
        out
    }

    pub fn node_id(&self) -> String {
        skill_node_id(&self.meta.name)
    }
}

pub fn skill_node_id(name: &str) -> String {
    let mut slug = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    let slug = slug.trim_matches('_').to_string();
    if slug.is_empty() {
        SKILLS_ROOT_NODE.to_string()
    } else {
        format!("{SKILLS_ROOT_NODE}.{slug}")
    }
}

pub async fn write_skill(mem: &dyn MemoireCognitive, skill: &Skill) -> Result<Value> {
    mem.write(
        MemoryItem::new(skill.node_id(), skill.to_markdown())
            .with_source("skill-okf")
            .with_tags(vec!["skill".to_string(), "okf".to_string()]),
    )
    .await
}

pub async fn propose_skill(mem: &dyn MemoireCognitive, skill: &Skill) -> Result<Value> {
    mem.propose_write(
        MemoryItem::new(skill.node_id(), skill.to_markdown())
            .with_source("auto-skill")
            .with_tags(vec!["skill".to_string(), "okf".to_string()]),
    )
    .await
}

pub async fn read_skill(mem: &dyn MemoireCognitive, name: &str) -> Result<Option<Skill>> {
    let node_id = skill_node_id(name);
    let node = mem.read_node(&node_id).await?;
    let Some(items) = node.get("items").and_then(Value::as_array) else {
        return Ok(None);
    };
    for item in items.iter().rev() {
        if let Some(content) = item.get("content").and_then(Value::as_str) {
            if let Ok(skill) = Skill::parse(content) {
                return Ok(Some(skill));
            }
        }
    }
    Ok(None)
}

pub async fn list_skills(mem: &dyn MemoireCognitive, limit: Option<u8>) -> Result<Value> {
    let root = mem.read_node(SKILLS_ROOT_NODE).await?;
    let children = root
        .get("children")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !children.is_empty() {
        return Ok(json!({ "skills": children }));
    }
    Ok(mem
        .search(
            "type: skill",
            SearchOpts {
                depth: None,
                limit,
                sans_trace: false,
            },
        )
        .await?
        .raw)
}

fn split_frontmatter(markdown: &str) -> Result<(&str, &str)> {
    let s = markdown.trim_start_matches('\u{feff}');
    let rest = s
        .strip_prefix("---")
        .ok_or_else(|| anyhow!("missing OKF frontmatter"))?;
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))
        .unwrap_or(rest);
    let Some(marker) = rest.find("\n---") else {
        return Err(anyhow!("missing end of OKF frontmatter"));
    };
    let fm = &rest[..marker];
    let body = &rest[marker + "\n---".len()..];
    let body = body
        .strip_prefix("\r\n")
        .or_else(|| body.strip_prefix('\n'))
        .unwrap_or(body);
    Ok((fm, body))
}

fn parse_frontmatter(frontmatter: &str) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    let mut lignes = frontmatter.lines().peekable();
    while let Some(raw) = lignes.next() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(anyhow!("invalid YAML line: {line}"));
        };
        let value = value.trim();
        // YAML BLOCK SCALAR (`>-`, `|`, ...): the value is on the following indented
        // lines. Without this branch the marker line stored ">-" and the continuation
        // line, having no colon, aborted the whole parse: every skill written that way
        // was unreadable here while parsing fine in contexte.rs. Two parsers, one file
        // format, opposite answers. Keep this in step with `yaml_frontmatter_field`.
        if matches!(value, ">" | ">-" | ">+" | "|" | "|-" | "|+") {
            let plie = value.starts_with('>');
            let mut morceaux: Vec<String> = Vec::new();
            while let Some(suite) = lignes.peek() {
                if suite.trim().is_empty() || !suite.starts_with([' ', '\t']) {
                    break;
                }
                morceaux.push(suite.trim().to_string());
                lignes.next();
            }
            let joint = morceaux.join(if plie { " " } else { "\n" });
            out.insert(key.trim().to_string(), joint);
            continue;
        }
        out.insert(key.trim().to_string(), unquote(value));
    }
    Ok(out)
}

fn required(fields: &BTreeMap<String, String>, key: &str) -> Result<String> {
    fields
        .get(key)
        .filter(|v| !v.trim().is_empty())
        .cloned()
        .ok_or_else(|| anyhow!("missing required OKF field: {key}"))
}

fn parse_array(value: Option<&String>) -> Vec<String> {
    let Some(v) = value.map(|s| s.trim()) else {
        return Vec::new();
    };
    let inner = v.strip_prefix('[').and_then(|s| s.strip_suffix(']'));
    let Some(inner) = inner else {
        return v
            .split(',')
            .map(|s| unquote(s.trim()))
            .filter(|s| !s.is_empty())
            .collect();
    };
    inner
        .split(',')
        .map(|s| unquote(s.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_body(body: &str) -> (Vec<Paradigm>, Vec<Step>) {
    enum Current {
        Paradigm(Paradigm),
        Step(Step),
    }

    let mut paradigms = Vec::new();
    let mut steps = Vec::new();
    let mut current: Option<Current> = None;

    fn flush(current: &mut Option<Current>, paradigms: &mut Vec<Paradigm>, steps: &mut Vec<Step>) {
        match current.take() {
            Some(Current::Paradigm(p)) => paradigms.push(p),
            Some(Current::Step(s)) => steps.push(s),
            None => {}
        }
    }

    for raw in body.lines() {
        let line = raw.trim();
        if line.starts_with("## Paradigm:") || line.starts_with("## Paradigme:") {
            flush(&mut current, &mut paradigms, &mut steps);
            let title = line
                .trim_start_matches("## Paradigm:")
                .trim_start_matches("## Paradigme:")
                .trim()
                .to_string();
            current = Some(Current::Paradigm(Paradigm {
                id: format!("p_{}", paradigms.len() + 1),
                title,
                description: String::new(),
                rules: Vec::new(),
            }));
            continue;
        }
        if line.starts_with("## Step:") || line.starts_with("## Etape:") {
            flush(&mut current, &mut paradigms, &mut steps);
            let name = line
                .trim_start_matches("## Step:")
                .trim_start_matches("## Etape:")
                .trim()
                .to_string();
            current = Some(Current::Step(Step {
                id: format!("step_{}", steps.len() + 1),
                name,
                instruction: String::new(),
                success_criteria: Vec::new(),
                execution: None,
                artifacts: Vec::new(),
                human_checkpoint: false,
            }));
            continue;
        }
        if line.starts_with("# ") || line.starts_with('#') || line.is_empty() {
            continue;
        }

        match current.as_mut() {
            Some(Current::Paradigm(p)) if line.starts_with("- ") => {
                p.rules
                    .push(line.trim_start_matches("- ").trim().to_string());
            }
            Some(Current::Paradigm(p)) => append_line(&mut p.description, line),
            Some(Current::Step(s)) if line.starts_with("- ") => parse_step_bullet(s, line),
            Some(Current::Step(s)) => append_line(&mut s.instruction, line),
            None => {}
        }
    }
    flush(&mut current, &mut paradigms, &mut steps);
    (paradigms, steps)
}

fn parse_step_bullet(step: &mut Step, line: &str) {
    let item = line.trim_start_matches("- ").trim();
    if let Some(v) = item.strip_prefix("Execution:") {
        step.execution = Some(v.trim().to_string());
    } else if let Some(v) = item.strip_prefix("Artifacts:") {
        step.artifacts = v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    } else if item.to_lowercase().contains("human checkpoint") {
        step.human_checkpoint = true;
    } else if let Some(v) = item.strip_prefix("Criterion:") {
        step.success_criteria.push(v.trim().to_string());
    } else if let Some(v) = item.strip_prefix("Critere:") {
        step.success_criteria.push(v.trim().to_string());
    } else {
        step.success_criteria.push(item.to_string());
    }
}

fn append_line(target: &mut String, line: &str) {
    if !target.is_empty() {
        target.push('\n');
    }
    target.push_str(line);
}

fn yaml_scalar(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn yaml_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|v| yaml_scalar(v))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn unquote(value: &str) -> String {
    let v = value.trim();
    if v.len() >= 2
        && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')))
    {
        v[1..v.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests_frontmatter {
    use super::*;

    /// Regression: `description: >-` made `Skill::parse` return
    /// `Err(invalid YAML line: ...)`, because the folded continuation line carries no
    /// colon. Every skill written that way was silently dropped by the callers that
    /// swallow the error, so the dashboard listed node labels instead of descriptions.
    #[test]
    fn block_scalar_description_is_folded_not_rejected() {
        let md = "---\ntype: skill\nname: web-research\ndescription: >-\n  Answer a factual \
                  question from the web,\n  with cross-checked sources.\n---\n\n# Body\n";
        let skill = Skill::parse(md).expect("a folded description must parse");
        assert_eq!(
            skill.meta.description,
            "Answer a factual question from the web, with cross-checked sources."
        );
    }

    /// The form `skill_create` actually writes, and the form every shipped SKILL.md now
    /// uses. It must keep working exactly as before.
    #[test]
    fn plain_description_is_unchanged() {
        let md = "---\ntype: skill\nname: web-research\ndescription: Answer a factual \
                  question from the web, with cross-checked sources.\n---\n\n# Body\n";
        let skill = Skill::parse(md).expect("a plain description must parse");
        assert_eq!(
            skill.meta.description,
            "Answer a factual question from the web, with cross-checked sources."
        );
    }

    /// A literal block keeps its line breaks, a folded one joins with spaces.
    #[test]
    fn literal_block_keeps_line_breaks() {
        let md = "---\ntype: skill\nname: x\ndescription: |\n  one\n  two\n---\n\n# Body\n";
        let skill = Skill::parse(md).expect("a literal block must parse");
        assert_eq!(skill.meta.description, "one\ntwo");
    }

    /// `prerequisites:` with an indented `commands:` child must not abort the parse of
    /// the fields around it. Eleven shipped skills declare prerequisites this way.
    #[test]
    fn nested_prerequisites_do_not_break_the_parse() {
        let md = "---\ntype: skill\nname: openhue\ndescription: Control lights.\n\
                  prerequisites:\n  commands: [openhue, jq]\n---\n\n# Body\n";
        let skill = Skill::parse(md).expect("nested prerequisites must not abort");
        assert_eq!(skill.meta.name, "openhue");
        assert_eq!(skill.meta.description, "Control lights.");
    }
}
