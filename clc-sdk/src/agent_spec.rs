//! Declarative agent configuration.
//!
//! `AgentSpec` is a YAML-serializable struct that declaratively configures
//! an agent session. It can be parsed from standalone YAML or from markdown
//! frontmatter (where the body becomes the prompt).
//!
//! Consumers: missouri agent evals, tisket issue metadata, clc coordinator
//! scopes, and anywhere else agent configuration is specified declaratively.

use serde::Deserialize;

use crate::agent::AgentConfig;

/// Declarative agent configuration.
///
/// All fields are optional — unset fields inherit from the caller's defaults
/// via [`AgentSpec::to_agent_config`].
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentSpec {
    /// Model identifier (e.g. "haiku", "sonnet", "opus").
    #[serde(default)]
    pub model: Option<String>,

    /// Maximum agentic turns before the session is terminated.
    #[serde(default)]
    pub max_turns: Option<u32>,

    /// Maximum cost in cents before the session is terminated.
    #[serde(default)]
    pub max_cost_cents: Option<u32>,

    /// Additional CLI arguments passed to the agent binary.
    #[serde(default)]
    pub extra_args: Vec<String>,

    /// Tools the agent is allowed to use without permission prompts.
    #[serde(default)]
    pub allowed_tools: Vec<String>,
}

/// Defaults used when an `AgentSpec` field is unset.
#[derive(Debug, Clone)]
pub struct AgentDefaults {
    pub model: String,
    pub system_prompt: String,
    pub initial_prompt: String,
    pub extra_args: Vec<String>,
    pub allowed_tools: Vec<String>,
}

impl AgentSpec {
    /// Parse from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yml::Error> {
        serde_yml::from_str(yaml)
    }

    /// Parse from markdown with optional YAML frontmatter.
    ///
    /// Returns the parsed spec and the markdown body (everything after the
    /// closing `---`). If no frontmatter is present, the spec is all defaults
    /// and the entire content is returned as the body.
    pub fn from_markdown(content: &str) -> Result<(Self, String), serde_yml::Error> {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return Ok((Self::default(), content.to_string()));
        }

        // Find the closing --- after the opening one.
        let after_open = &trimmed[3..];
        let after_open = after_open.strip_prefix('\n').unwrap_or(after_open);

        // Handle empty frontmatter: closing --- is the very first thing.
        if after_open.starts_with("---") {
            let body = after_open[3..]
                .strip_prefix('\n')
                .unwrap_or(&after_open[3..]);
            let spec: AgentSpec = serde_yml::from_str("{}")?;
            return Ok((spec, body.to_string()));
        }

        if let Some(close_pos) = after_open.find("\n---") {
            let yaml_block = &after_open[..close_pos];
            let body_start = close_pos + 4; // skip "\n---"
            let body = after_open[body_start..]
                .strip_prefix('\n')
                .unwrap_or(&after_open[body_start..]);

            let spec: AgentSpec = serde_yml::from_str(yaml_block)?;
            Ok((spec, body.to_string()))
        } else {
            // Opening --- but no closing --- means no valid frontmatter.
            Ok((Self::default(), content.to_string()))
        }
    }

    /// Build an [`AgentConfig`] by overlaying this spec onto the given defaults.
    ///
    /// Spec fields that are `Some` override the corresponding default. `extra_args`
    /// from both the defaults and the spec are concatenated (defaults first).
    pub fn to_agent_config(&self, defaults: &AgentDefaults) -> AgentConfig {
        let model = self
            .model
            .clone()
            .unwrap_or_else(|| defaults.model.clone());

        let mut extra_args = defaults.extra_args.clone();
        // max_turns: no Claude Code CLI flag exists for this yet.
        // The field is preserved in the spec for future use / other agents.
        if let Some(cents) = self.max_cost_cents {
            // Claude Code uses --max-budget-usd in dollars; convert from cents.
            let dollars = f64::from(cents) / 100.0;
            extra_args.push("--max-budget-usd".to_string());
            extra_args.push(format!("{dollars:.2}"));
        }
        extra_args.extend(self.extra_args.clone());

        // Merge allowed_tools: defaults first, then spec additions.
        let mut allowed_tools = defaults.allowed_tools.clone();
        allowed_tools.extend(self.allowed_tools.clone());

        AgentConfig {
            model,
            system_prompt: defaults.system_prompt.clone(),
            initial_prompt: defaults.initial_prompt.clone(),
            extra_args,
            allowed_tools,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- from_yaml ----

    #[test]
    fn from_yaml_all_fields() {
        let yaml = r#"
model: haiku
max_turns: 5
max_cost_cents: 50
extra_args:
  - --verbose
  - --no-cache
"#;
        let spec = AgentSpec::from_yaml(yaml).unwrap();
        assert_eq!(spec.model.as_deref(), Some("haiku"));
        assert_eq!(spec.max_turns, Some(5));
        assert_eq!(spec.max_cost_cents, Some(50));
        assert_eq!(spec.extra_args, vec!["--verbose", "--no-cache"]);
    }

    #[test]
    fn from_yaml_empty() {
        let spec = AgentSpec::from_yaml("{}").unwrap();
        assert!(spec.model.is_none());
        assert!(spec.max_turns.is_none());
        assert!(spec.max_cost_cents.is_none());
        assert!(spec.extra_args.is_empty());
    }

    #[test]
    fn from_yaml_partial_model_only() {
        let spec = AgentSpec::from_yaml("model: opus").unwrap();
        assert_eq!(spec.model.as_deref(), Some("opus"));
        assert!(spec.max_turns.is_none());
        assert!(spec.max_cost_cents.is_none());
        assert!(spec.extra_args.is_empty());
    }

    #[test]
    fn from_yaml_partial_budget_only() {
        let yaml = "max_cost_cents: 200";
        let spec = AgentSpec::from_yaml(yaml).unwrap();
        assert!(spec.model.is_none());
        assert_eq!(spec.max_cost_cents, Some(200));
        assert!(spec.extra_args.is_empty());
    }

    // ---- from_markdown ----

    #[test]
    fn from_markdown_with_frontmatter() {
        let md = r#"---
model: haiku
max_turns: 5
max_cost_cents: 50
---

Verify that every CLI command mentioned in this skill file
exists on PATH and accepts the flags shown.
"#;
        let (spec, body) = AgentSpec::from_markdown(md).unwrap();
        assert_eq!(spec.model.as_deref(), Some("haiku"));
        assert_eq!(spec.max_turns, Some(5));
        assert_eq!(spec.max_cost_cents, Some(50));
        assert!(body.contains("Verify that every CLI command"));
        assert!(!body.contains("---"));
    }

    #[test]
    fn from_markdown_no_frontmatter() {
        let md = "Just a plain markdown document.\n\nNo frontmatter here.";
        let (spec, body) = AgentSpec::from_markdown(md).unwrap();
        assert!(spec.model.is_none());
        assert!(spec.max_turns.is_none());
        assert_eq!(body, md);
    }

    #[test]
    fn from_markdown_empty_frontmatter() {
        let md = "---\n---\nBody after empty frontmatter.";
        let (spec, body) = AgentSpec::from_markdown(md).unwrap();
        assert!(spec.model.is_none());
        assert!(spec.max_turns.is_none());
        assert_eq!(body, "Body after empty frontmatter.");
    }

    #[test]
    fn from_markdown_unclosed_frontmatter() {
        let md = "---\nmodel: haiku\nThis never closes the frontmatter.";
        let (spec, body) = AgentSpec::from_markdown(md).unwrap();
        // No closing ---, so treated as no frontmatter.
        assert!(spec.model.is_none());
        assert_eq!(body, md);
    }

    // ---- to_agent_config ----

    fn test_defaults() -> AgentDefaults {
        AgentDefaults {
            model: "sonnet".to_string(),
            system_prompt: "default system prompt".to_string(),
            initial_prompt: "default initial prompt".to_string(),
            extra_args: vec!["--default-flag".to_string()],
            allowed_tools: vec![],
        }
    }

    #[test]
    fn to_agent_config_overrides_model() {
        let spec = AgentSpec {
            model: Some("opus".to_string()),
            ..Default::default()
        };
        let config = spec.to_agent_config(&test_defaults());
        assert_eq!(config.model, "opus");
        // system/initial prompts come from defaults
        assert_eq!(config.system_prompt, "default system prompt");
        assert_eq!(config.initial_prompt, "default initial prompt");
    }

    #[test]
    fn to_agent_config_none_falls_through() {
        let spec = AgentSpec::default();
        let config = spec.to_agent_config(&test_defaults());
        assert_eq!(config.model, "sonnet");
        assert_eq!(config.extra_args, vec!["--default-flag"]);
    }

    #[test]
    fn to_agent_config_extra_args_concatenated() {
        let spec = AgentSpec {
            extra_args: vec!["--spec-flag".to_string()],
            ..Default::default()
        };
        let config = spec.to_agent_config(&test_defaults());
        assert_eq!(config.extra_args, vec!["--default-flag", "--spec-flag"]);
    }

    #[test]
    fn to_agent_config_budget_becomes_arg() {
        let spec = AgentSpec {
            max_cost_cents: Some(100),
            ..Default::default()
        };
        let config = spec.to_agent_config(&test_defaults());
        assert!(config.extra_args.contains(&"--max-budget-usd".to_string()));
        assert!(config.extra_args.contains(&"1.00".to_string()));
    }

    #[test]
    fn to_agent_config_max_turns_not_emitted() {
        let spec = AgentSpec {
            max_turns: Some(10),
            ..Default::default()
        };
        let config = spec.to_agent_config(&test_defaults());
        // max_turns has no CLI flag yet — should not appear in extra_args
        assert!(!config.extra_args.iter().any(|a| a.contains("turns")));
    }

    #[test]
    fn to_agent_config_allowed_tools_merged() {
        let mut defaults = test_defaults();
        defaults.allowed_tools = vec!["Read".to_string(), "Glob".to_string()];

        let spec = AgentSpec {
            allowed_tools: vec!["Bash(missouri agent*)".to_string()],
            ..Default::default()
        };
        let config = spec.to_agent_config(&defaults);
        assert_eq!(
            config.allowed_tools,
            vec!["Read", "Glob", "Bash(missouri agent*)"]
        );
    }

    #[test]
    fn to_agent_config_allowed_tools_empty_by_default() {
        let spec = AgentSpec::default();
        let config = spec.to_agent_config(&test_defaults());
        assert!(config.allowed_tools.is_empty());
    }

    #[test]
    fn from_yaml_allowed_tools() {
        let yaml = r#"
model: haiku
allowed_tools:
  - Read
  - "Bash(npm test*)"
"#;
        let spec = AgentSpec::from_yaml(yaml).unwrap();
        assert_eq!(spec.allowed_tools, vec!["Read", "Bash(npm test*)"]);
    }
}
