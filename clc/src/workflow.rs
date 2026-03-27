use std::collections::HashMap;

use crate::config::{PhaseDef, PermissionsDef, ReviewDef, TransitionDef, WorkflowDef};
use crate::error::Error;

/// A validated workflow graph. Constructed from a `WorkflowDef`, with all
/// transition targets verified to exist in the phase list.
#[derive(Debug, Clone)]
pub struct Workflow {
    /// Workflow description injected into prime text.
    description: Option<String>,
    /// Ordered phase definitions, indexed by name.
    phases: Vec<PhaseDef>,
    /// Phase name → index in `phases` for O(1) lookup.
    index: HashMap<String, usize>,
    /// Review type definitions (consumed by review gate runtime, not yet implemented).
    #[allow(dead_code)]
    reviews: HashMap<String, ReviewDef>,
}

impl Workflow {
    /// Build a validated workflow from a config definition.
    ///
    /// Returns an error if:
    /// - The workflow has no phases
    /// - A transition references a nonexistent phase
    /// - A transition `requires` references a nonexistent review type
    /// - Duplicate phase names exist
    pub fn new(def: &WorkflowDef) -> Result<Self, Error> {
        if def.phases.is_empty() {
            return Err(Error::NonBlocking(
                "workflow has no phases".to_string(),
            ));
        }

        let mut index = HashMap::new();
        for (i, phase) in def.phases.iter().enumerate() {
            if index.insert(phase.name.clone(), i).is_some() {
                return Err(Error::NonBlocking(format!(
                    "duplicate phase name: '{}'",
                    phase.name
                )));
            }
        }

        // Validate all transition targets and review references.
        for phase in &def.phases {
            if let Some(transitions) = &phase.transitions {
                for t in transitions {
                    if !index.contains_key(t.target()) {
                        return Err(Error::NonBlocking(format!(
                            "phase '{}' has transition to unknown phase '{}'",
                            phase.name,
                            t.target()
                        )));
                    }
                    for req in t.requires() {
                        if !def.reviews.contains_key(req) {
                            return Err(Error::NonBlocking(format!(
                                "phase '{}' transition to '{}' requires unknown review type '{req}'",
                                phase.name,
                                t.target()
                            )));
                        }
                    }
                }
            }
        }

        Ok(Self {
            description: def.description.clone(),
            phases: def.phases.clone(),
            index,
            reviews: def.reviews.clone(),
        })
    }

    /// The built-in TDD workflow that replicates the current hardcoded behavior.
    pub fn default_tdd() -> Self {
        let test_only = Some(PermissionsDef {
            allow: vec![
                "Edit(tests/**)".into(),
                "Write(tests/**)".into(),
                "Bash(cargo test *)".into(),
                "Bash(missouri *)".into(),
            ],
            deny: vec!["Edit".into(), "Write".into(), "Bash".into()],
        });

        let def = WorkflowDef {
            description: Some(
                "Test-driven development. Write failing tests, implement until \
                 green, request code review, finalize."
                    .into(),
            ),
            phases: vec![
                PhaseDef {
                    name: "tests-unwritten".into(),
                    instructions: Some("Write failing tests that specify the desired behavior.".into()),
                    nudge: None,
                    can_stop: false,
                    permissions: test_only.clone(),
                    transitions: Some(vec![TransitionDef::Simple("tests-written".into())]),
                },
                PhaseDef {
                    name: "tests-written".into(),
                    instructions: Some("Verify tests fail for the right reasons.".into()),
                    nudge: None,
                    can_stop: false,
                    permissions: test_only.clone(),
                    transitions: Some(vec![
                        TransitionDef::Simple("red".into()),
                        TransitionDef::Simple("tests-unwritten".into()),
                    ]),
                },
                PhaseDef {
                    name: "red".into(),
                    instructions: Some("Tests are red. Confirm failures match expectations.".into()),
                    nudge: None,
                    can_stop: false,
                    permissions: test_only.clone(),
                    transitions: Some(vec![
                        TransitionDef::Simple("implementing".into()),
                        TransitionDef::Simple("tests-unwritten".into()),
                    ]),
                },
                PhaseDef {
                    name: "implementing".into(),
                    instructions: Some("Write the minimum code to make failing tests pass.".into()),
                    nudge: Some("Run tests to check your progress.".into()),
                    can_stop: false,
                    permissions: None, // unrestricted
                    transitions: Some(vec![
                        TransitionDef::Simple("green".into()),
                        TransitionDef::Simple("red".into()),
                    ]),
                },
                PhaseDef {
                    name: "green".into(),
                    instructions: Some("Tests pass. Refactor if needed.".into()),
                    nudge: None,
                    can_stop: true,
                    permissions: test_only.clone(),
                    transitions: Some(vec![
                        TransitionDef::Simple("implementing".into()),
                        TransitionDef::Simple("review-requested".into()),
                    ]),
                },
                PhaseDef {
                    name: "review-requested".into(),
                    instructions: Some("Review has been requested.".into()),
                    nudge: None,
                    can_stop: true,
                    permissions: test_only.clone(),
                    transitions: Some(vec![TransitionDef::Simple("in-review".into())]),
                },
                PhaseDef {
                    name: "in-review".into(),
                    instructions: Some("Work is being reviewed.".into()),
                    nudge: None,
                    can_stop: false,
                    permissions: None, // unrestricted (reviewer needs to run tools)
                    transitions: Some(vec![TransitionDef::Simple("reviewed".into())]),
                },
                PhaseDef {
                    name: "reviewed".into(),
                    instructions: Some("Review complete.".into()),
                    nudge: None,
                    can_stop: true,
                    permissions: test_only,
                    transitions: Some(vec![
                        TransitionDef::Simple("done".into()),
                        TransitionDef::Simple("implementing".into()),
                    ]),
                },
                PhaseDef {
                    name: "done".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false, // terminal — stop check is different
                    permissions: None,
                    transitions: None, // terminal
                },
            ],
            reviews: HashMap::new(),
        };

        // The default TDD workflow is known-valid; unwrap is safe.
        Self::new(&def).expect("built-in TDD workflow is invalid")
    }

    /// Workflow description for prime text injection.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The initial phase name (first in the list).
    pub fn initial_phase(&self) -> &str {
        &self.phases[0].name
    }

    /// Whether this phase is terminal (no outgoing transitions).
    pub fn is_terminal(&self, phase: &str) -> bool {
        self.phase_def(phase)
            .map_or(false, |p| p.transitions.is_none())
    }

    /// Whether the agent can stop at this phase.
    /// Terminal phases always allow stopping.
    pub fn can_stop(&self, phase: &str) -> bool {
        self.phase_def(phase)
            .map_or(false, |p| p.can_stop || p.transitions.is_none())
    }

    /// Whether `from → to` is a valid forward transition (declared edge).
    pub fn can_transition(&self, from: &str, to: &str) -> bool {
        self.phase_def(from).map_or(false, |p| {
            p.transitions
                .as_ref()
                .map_or(false, |ts| ts.iter().any(|t| t.target() == to))
        })
    }

    /// Whether `from → to` is a backward transition (to an earlier phase in
    /// the definition order). Backward transitions are always valid.
    pub fn is_backward(&self, from: &str, to: &str) -> bool {
        match (self.index.get(from), self.index.get(to)) {
            (Some(&fi), Some(&ti)) => ti < fi,
            _ => false,
        }
    }

    /// Whether `from → to` is a valid move: either a declared forward edge
    /// or a backward transition.
    pub fn is_valid_transition(&self, from: &str, to: &str) -> bool {
        self.can_transition(from, to) || self.is_backward(from, to)
    }

    /// Permissions for a phase. `None` means unrestricted.
    pub fn phase_permissions(&self, phase: &str) -> Option<&PermissionsDef> {
        self.phase_def(phase).and_then(|p| p.permissions.as_ref())
    }

    /// Instructions for a phase (injected into prime text).
    pub fn phase_instructions(&self, phase: &str) -> Option<&str> {
        self.phase_def(phase)
            .and_then(|p| p.instructions.as_deref())
    }

    /// Nudge text for a phase (post-tool reminder).
    pub fn phase_nudge(&self, phase: &str) -> Option<&str> {
        self.phase_def(phase).and_then(|p| p.nudge.as_deref())
    }

    /// Review types required for a specific transition.
    pub fn transition_requires(&self, from: &str, to: &str) -> Option<&[String]> {
        let phase = self.phase_def(from)?;
        let transitions = phase.transitions.as_ref()?;
        transitions
            .iter()
            .find(|t| t.target() == to)
            .map(|t| t.requires())
            .filter(|r| !r.is_empty())
    }

    /// Get a review definition by type name.
    #[allow(dead_code)] // Consumed by review gate runtime, not yet implemented
    pub fn review_def(&self, review_type: &str) -> Option<&ReviewDef> {
        self.reviews.get(review_type)
    }

    /// Whether a phase name exists in this workflow.
    pub fn has_phase(&self, phase: &str) -> bool {
        self.index.contains_key(phase)
    }

    /// All phase names in order.
    pub fn phase_names(&self) -> impl Iterator<Item = &str> {
        self.phases.iter().map(|p| p.name.as_str())
    }

    /// Look up a phase definition by name.
    fn phase_def(&self, name: &str) -> Option<&PhaseDef> {
        self.index.get(name).map(|&i| &self.phases[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tdd() -> Workflow {
        Workflow::default_tdd()
    }

    // --- default_tdd parity tests ---

    #[test]
    fn default_tdd_has_nine_phases() {
        let wf = tdd();
        assert_eq!(wf.phases.len(), 9);
    }

    #[test]
    fn default_tdd_initial_phase() {
        assert_eq!(tdd().initial_phase(), "tests-unwritten");
    }

    #[test]
    fn default_tdd_terminal_phase() {
        let wf = tdd();
        assert!(wf.is_terminal("done"));
        assert!(!wf.is_terminal("implementing"));
        assert!(!wf.is_terminal("green"));
    }

    #[test]
    fn default_tdd_can_stop() {
        let wf = tdd();
        assert!(wf.can_stop("done")); // terminal
        assert!(wf.can_stop("review-requested"));
        assert!(wf.can_stop("reviewed"));
        assert!(wf.can_stop("green"));
        assert!(!wf.can_stop("tests-unwritten"));
        assert!(!wf.can_stop("implementing"));
        assert!(!wf.can_stop("in-review"));
    }

    #[test]
    fn default_tdd_forward_transitions() {
        let wf = tdd();
        assert!(wf.can_transition("tests-unwritten", "tests-written"));
        assert!(wf.can_transition("tests-written", "red"));
        assert!(wf.can_transition("red", "implementing"));
        assert!(wf.can_transition("implementing", "green"));
        assert!(wf.can_transition("green", "review-requested"));
        assert!(wf.can_transition("review-requested", "in-review"));
        assert!(wf.can_transition("in-review", "reviewed"));
        assert!(wf.can_transition("reviewed", "done"));
    }

    #[test]
    fn default_tdd_no_skip_forward() {
        let wf = tdd();
        assert!(!wf.can_transition("tests-unwritten", "red"));
        assert!(!wf.can_transition("tests-unwritten", "implementing"));
        assert!(!wf.can_transition("implementing", "done"));
    }

    #[test]
    fn default_tdd_backward_transitions() {
        let wf = tdd();
        assert!(wf.is_backward("implementing", "red"));
        assert!(wf.is_backward("implementing", "tests-unwritten"));
        assert!(wf.is_backward("green", "implementing"));
        assert!(wf.is_backward("reviewed", "implementing"));
        assert!(!wf.is_backward("tests-unwritten", "implementing"));
        assert!(!wf.is_backward("red", "red"));
    }

    #[test]
    fn default_tdd_valid_transitions() {
        let wf = tdd();
        // Forward
        assert!(wf.is_valid_transition("tests-unwritten", "tests-written"));
        // Backward
        assert!(wf.is_valid_transition("implementing", "tests-unwritten"));
        // Invalid (skip forward)
        assert!(!wf.is_valid_transition("tests-unwritten", "implementing"));
    }

    #[test]
    fn default_tdd_unrestricted_phases() {
        let wf = tdd();
        assert!(wf.phase_permissions("implementing").is_none());
        assert!(wf.phase_permissions("in-review").is_none());
    }

    #[test]
    fn default_tdd_restricted_phases() {
        let wf = tdd();
        assert!(wf.phase_permissions("tests-unwritten").is_some());
        assert!(wf.phase_permissions("green").is_some());
        assert!(wf.phase_permissions("review-requested").is_some());
    }

    #[test]
    fn default_tdd_nudge() {
        let wf = tdd();
        assert!(wf.phase_nudge("implementing").is_some());
        assert!(wf.phase_nudge("tests-unwritten").is_none());
        assert!(wf.phase_nudge("green").is_none());
    }

    #[test]
    fn default_tdd_instructions() {
        let wf = tdd();
        assert!(wf.phase_instructions("tests-unwritten").is_some());
        assert!(wf.phase_instructions("implementing").is_some());
        assert!(wf.phase_instructions("done").is_none());
    }

    #[test]
    fn default_tdd_description() {
        assert!(tdd().description().is_some());
    }

    // --- Custom workflow tests ---

    fn docs_workflow() -> Workflow {
        let def = WorkflowDef {
            description: Some("Documentation writing.".into()),
            phases: vec![
                PhaseDef {
                    name: "outline".into(),
                    instructions: Some("Establish structure.".into()),
                    nudge: None,
                    can_stop: false,
                    permissions: Some(PermissionsDef {
                        allow: vec!["Edit(docs/**)".into()],
                        deny: vec!["Edit".into(), "Write".into()],
                    }),
                    transitions: Some(vec![TransitionDef::Simple("draft".into())]),
                },
                PhaseDef {
                    name: "draft".into(),
                    instructions: Some("Write sections.".into()),
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: Some(vec![
                        TransitionDef::Simple("outline".into()),
                        TransitionDef::Rich {
                            target: "done".into(),
                            requires: vec!["writing".into()],
                        },
                    ]),
                },
                PhaseDef {
                    name: "done".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: None,
                },
            ],
            reviews: {
                let mut m = HashMap::new();
                m.insert(
                    "writing".into(),
                    ReviewDef {
                        instructions: Some("Review for clarity.".into()),
                        permissions: Some(PermissionsDef {
                            allow: vec![],
                            deny: vec!["Edit".into(), "Write".into()],
                        }),
                    },
                );
                m
            },
        };
        Workflow::new(&def).unwrap()
    }

    #[test]
    fn custom_workflow_initial_phase() {
        assert_eq!(docs_workflow().initial_phase(), "outline");
    }

    #[test]
    fn custom_workflow_terminal() {
        let wf = docs_workflow();
        assert!(wf.is_terminal("done"));
        assert!(!wf.is_terminal("draft"));
    }

    #[test]
    fn custom_workflow_transitions() {
        let wf = docs_workflow();
        assert!(wf.can_transition("outline", "draft"));
        assert!(wf.can_transition("draft", "done"));
        assert!(!wf.can_transition("outline", "done"));
    }

    #[test]
    fn custom_workflow_backward() {
        let wf = docs_workflow();
        assert!(wf.is_backward("draft", "outline"));
        assert!(!wf.is_backward("outline", "draft"));
    }

    #[test]
    fn custom_workflow_review_gate() {
        let wf = docs_workflow();
        let reqs = wf.transition_requires("draft", "done");
        assert_eq!(reqs, Some(&["writing".to_string()][..]));

        // Simple transition has no requirements
        assert!(wf.transition_requires("outline", "draft").is_none());
    }

    #[test]
    fn custom_workflow_review_def() {
        let wf = docs_workflow();
        let review = wf.review_def("writing").unwrap();
        assert_eq!(review.instructions.as_deref(), Some("Review for clarity."));
        assert!(wf.review_def("nonexistent").is_none());
    }

    #[test]
    fn custom_workflow_permissions() {
        let wf = docs_workflow();
        let perms = wf.phase_permissions("outline").unwrap();
        assert_eq!(perms.allow, vec!["Edit(docs/**)"]);
        assert!(wf.phase_permissions("draft").is_none()); // unrestricted
    }

    #[test]
    fn custom_workflow_has_phase() {
        let wf = docs_workflow();
        assert!(wf.has_phase("outline"));
        assert!(wf.has_phase("draft"));
        assert!(!wf.has_phase("implementing"));
    }

    #[test]
    fn custom_workflow_phase_names() {
        let wf = docs_workflow();
        let names: Vec<&str> = wf.phase_names().collect();
        assert_eq!(names, vec!["outline", "draft", "done"]);
    }

    // --- Validation error tests ---

    #[test]
    fn validation_rejects_empty_workflow() {
        let def = WorkflowDef::default();
        assert!(Workflow::new(&def).is_err());
    }

    #[test]
    fn validation_rejects_duplicate_phase_names() {
        let def = WorkflowDef {
            description: None,
            phases: vec![
                PhaseDef {
                    name: "foo".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: None,
                },
                PhaseDef {
                    name: "foo".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: None,
                },
            ],
            reviews: HashMap::new(),
        };
        let err = Workflow::new(&def).unwrap_err();
        assert!(err.to_string().contains("duplicate phase name"));
    }

    #[test]
    fn validation_rejects_transition_to_unknown_phase() {
        let def = WorkflowDef {
            description: None,
            phases: vec![PhaseDef {
                name: "start".into(),
                instructions: None,
                nudge: None,
                can_stop: false,
                permissions: None,
                transitions: Some(vec![TransitionDef::Simple("nonexistent".into())]),
            }],
            reviews: HashMap::new(),
        };
        let err = Workflow::new(&def).unwrap_err();
        assert!(err.to_string().contains("unknown phase 'nonexistent'"));
    }

    #[test]
    fn validation_rejects_unknown_review_type_in_requires() {
        let def = WorkflowDef {
            description: None,
            phases: vec![
                PhaseDef {
                    name: "working".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: Some(vec![TransitionDef::Rich {
                        target: "done".into(),
                        requires: vec!["nonexistent-review".into()],
                    }]),
                },
                PhaseDef {
                    name: "done".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: None,
                },
            ],
            reviews: HashMap::new(),
        };
        let err = Workflow::new(&def).unwrap_err();
        assert!(err.to_string().contains("unknown review type"));
    }

    #[test]
    fn can_stop_on_terminal_even_without_flag() {
        let def = WorkflowDef {
            description: None,
            phases: vec![
                PhaseDef {
                    name: "working".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: Some(vec![TransitionDef::Simple("done".into())]),
                },
                PhaseDef {
                    name: "done".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false, // not set, but terminal
                    permissions: None,
                    transitions: None,
                },
            ],
            reviews: HashMap::new(),
        };
        let wf = Workflow::new(&def).unwrap();
        assert!(wf.can_stop("done"));
        assert!(!wf.can_stop("working"));
    }

    #[test]
    fn unknown_phase_returns_none_for_queries() {
        let wf = tdd();
        assert!(wf.phase_permissions("nonexistent").is_none());
        assert!(wf.phase_instructions("nonexistent").is_none());
        assert!(wf.phase_nudge("nonexistent").is_none());
        assert!(!wf.can_stop("nonexistent"));
        assert!(!wf.is_terminal("nonexistent"));
        assert!(!wf.can_transition("nonexistent", "done"));
        assert!(!wf.is_backward("nonexistent", "done"));
    }
}
