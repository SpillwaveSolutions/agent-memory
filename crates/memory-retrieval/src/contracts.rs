//! Skill contracts and explainability for retrieval operations.
//!
//! This module implements:
//! - `ExplainabilityPayload`: Detailed explanation of retrieval decisions
//! - `SkillContract`: Requirements that retrieval-capable skills must meet
//! - Validation functions for skill compliance
//!
//! Per PRD Section 8: Skill Contract (Normative)

use serde::{Deserialize, Serialize};

use crate::executor::ExecutionResult;
use crate::types::{CapabilityTier, ExecutionMode, QueryIntent, RetrievalLayer, StopConditions};

/// Explainability payload for retrieval decisions.
///
/// Per PRD Section 8: Skills must provide this information about
/// how and why the retrieval was performed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainabilityPayload {
    /// Classified query intent
    pub intent: QueryIntent,

    /// Detected capability tier
    pub tier: CapabilityTier,

    /// Execution mode used
    pub mode: ExecutionMode,

    /// Layers that were considered
    pub candidates_considered: Vec<RetrievalLayer>,

    /// Layer that ultimately provided results
    pub winner: RetrievalLayer,

    /// Explanation of why the winner was chosen
    pub why_winner: String,

    /// Whether fallback occurred
    pub fallback_occurred: bool,

    /// If fallback, why?
    pub fallback_reason: Option<String>,

    /// Stop conditions that were applied
    pub stop_conditions: StopConditions,

    /// Bounds that were hit (if any)
    pub bounds_hit: Vec<BoundHit>,

    /// Total retrieval time in milliseconds
    pub total_time_ms: u64,

    /// Number of results returned
    pub result_count: usize,

    /// Grip IDs in results (for evidence provenance)
    pub grip_ids: Vec<String>,
}

/// Record of a bound being hit during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundHit {
    /// Which bound was hit
    pub bound_type: BoundType,

    /// Configured limit
    pub limit: u64,

    /// Actual value when hit
    pub actual: u64,

    /// Action taken when bound was hit
    pub action: BoundAction,
}

/// Types of bounds that can be hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundType {
    /// Maximum depth
    MaxDepth,
    /// Maximum nodes visited
    MaxNodes,
    /// Maximum RPC calls
    MaxRpcCalls,
    /// Maximum tokens
    MaxTokens,
    /// Timeout
    Timeout,
    /// Beam width
    BeamWidth,
}

/// Action taken when a bound is hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundAction {
    /// Stopped immediately
    HardStop,
    /// Continued slightly past bound
    SoftExceed,
    /// Returned partial results
    PartialResults,
}

impl ExplainabilityPayload {
    /// Create a payload from execution result.
    pub fn from_execution(
        intent: QueryIntent,
        result: &ExecutionResult,
        conditions: &StopConditions,
    ) -> Self {
        // Extract grip IDs from results
        let grip_ids: Vec<String> = result
            .results
            .iter()
            .filter(|r| r.doc_type == "grip")
            .map(|r| r.doc_id.clone())
            .collect();

        let fallback_reason = if result.fallback_occurred {
            Some(result.explanation.clone())
        } else {
            None
        };

        Self {
            intent,
            tier: result.tier,
            mode: result.mode,
            candidates_considered: result.layers_attempted.clone(),
            winner: result.primary_layer,
            why_winner: result.explanation.clone(),
            fallback_occurred: result.fallback_occurred,
            fallback_reason,
            stop_conditions: conditions.clone(),
            bounds_hit: vec![], // Populated by executor if needed
            total_time_ms: result.total_time_ms,
            result_count: result.results.len(),
            grip_ids,
        }
    }

    /// Create a minimal payload for when no retrieval was needed.
    pub fn minimal(tier: CapabilityTier) -> Self {
        Self {
            intent: QueryIntent::Answer,
            tier,
            mode: ExecutionMode::Sequential,
            candidates_considered: vec![],
            winner: RetrievalLayer::Agentic,
            why_winner: "No retrieval needed".to_string(),
            fallback_occurred: false,
            fallback_reason: None,
            stop_conditions: StopConditions::default(),
            bounds_hit: vec![],
            total_time_ms: 0,
            result_count: 0,
            grip_ids: vec![],
        }
    }

    /// Convert to a user-friendly summary string.
    pub fn to_summary(&self) -> String {
        let mut parts = Vec::new();

        parts.push(format!("Tier: {}", self.tier.description()));
        parts.push(format!(
            "Method: {} ({})",
            self.winner.as_str(),
            self.mode.as_str()
        ));

        if self.fallback_occurred {
            if let Some(ref reason) = self.fallback_reason {
                parts.push(format!("Fallback: {}", reason));
            } else {
                parts.push("Fallback occurred".to_string());
            }
        }

        parts.push(format!(
            "Results: {} in {}ms",
            self.result_count, self.total_time_ms
        ));

        if !self.grip_ids.is_empty() {
            parts.push(format!("Evidence: {} grips", self.grip_ids.len()));
        }

        parts.join(" | ")
    }

    /// Convert to markdown format for inclusion in responses.
    pub fn to_markdown(&self) -> String {
        let mut lines = Vec::new();

        lines.push("## Retrieval Method".to_string());
        lines.push(String::new());
        lines.push(format!("- **Tier:** {}", self.tier.description()));
        lines.push(format!("- **Intent:** {}", self.intent.as_str()));
        lines.push(format!("- **Mode:** {}", self.mode.as_str()));
        lines.push(format!("- **Method:** {}", self.winner.as_str()));

        if self.fallback_occurred {
            lines.push(format!(
                "- **Fallback:** {}",
                self.fallback_reason.as_deref().unwrap_or("Yes")
            ));
        }

        lines.push(String::new());
        lines.push("### Candidates Considered".to_string());
        for layer in &self.candidates_considered {
            let marker = if *layer == self.winner { "**" } else { "" };
            lines.push(format!("- {}{}{}", marker, layer.as_str(), marker));
        }

        lines.push(String::new());
        lines.push(format!(
            "*Found {} results in {}ms*",
            self.result_count, self.total_time_ms
        ));

        lines.join("\n")
    }
}

/// Skill contract requirements.
///
/// Per PRD Section 8: What every retrieval-capable skill MUST provide.
#[derive(Debug, Clone)]
pub struct SkillContract {
    /// Skill name
    pub name: String,

    /// Whether capability detection is performed
    pub performs_capability_detection: bool,

    /// Whether budget is enforced
    pub enforces_budget: bool,

    /// Whether fallback discipline is followed
    pub has_fallback_discipline: bool,

    /// Whether explainability payload is provided
    pub provides_explainability: bool,

    /// Whether evidence (grip_ids) is included
    pub handles_evidence: bool,

    /// Retrieval layers used
    pub layers_used: Vec<RetrievalLayer>,

    /// Custom stop conditions (beyond defaults)
    pub custom_stop_conditions: Option<StopConditions>,
}

impl SkillContract {
    /// Create a new skill contract.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            performs_capability_detection: false,
            enforces_budget: false,
            has_fallback_discipline: false,
            provides_explainability: false,
            handles_evidence: false,
            layers_used: vec![],
            custom_stop_conditions: None,
        }
    }

    /// Validate that the contract meets all requirements.
    pub fn validate(&self) -> SkillContractValidation {
        let mut issues = Vec::new();

        if !self.performs_capability_detection {
            issues.push(SkillContractIssue {
                requirement: "Capability Detection".to_string(),
                severity: IssueSeverity::Error,
                message: "Skill must check status RPCs once per request".to_string(),
            });
        }

        if !self.enforces_budget {
            issues.push(SkillContractIssue {
                requirement: "Budget Enforcement".to_string(),
                severity: IssueSeverity::Error,
                message: "Skill must respect max_rpc_calls, token_budget, timeout".to_string(),
            });
        }

        if !self.has_fallback_discipline {
            issues.push(SkillContractIssue {
                requirement: "Fallback Discipline".to_string(),
                severity: IssueSeverity::Error,
                message: "Skill must never hard-fail if agentic TOC search can run".to_string(),
            });
        }

        if !self.provides_explainability {
            issues.push(SkillContractIssue {
                requirement: "Explainability Payload".to_string(),
                severity: IssueSeverity::Warning,
                message: "Skill should report tier, mode, candidates, why winner won".to_string(),
            });
        }

        if !self.handles_evidence {
            issues.push(SkillContractIssue {
                requirement: "Evidence Handling".to_string(),
                severity: IssueSeverity::Warning,
                message: "Skill should include grip_ids/citations when returning facts".to_string(),
            });
        }

        let is_valid = !issues.iter().any(|i| i.severity == IssueSeverity::Error);

        SkillContractValidation {
            skill_name: self.name.clone(),
            is_valid,
            issues,
        }
    }

    /// Mark as having capability detection.
    pub fn with_capability_detection(mut self) -> Self {
        self.performs_capability_detection = true;
        self
    }

    /// Mark as enforcing budget.
    pub fn with_budget_enforcement(mut self) -> Self {
        self.enforces_budget = true;
        self
    }

    /// Mark as having fallback discipline.
    pub fn with_fallback_discipline(mut self) -> Self {
        self.has_fallback_discipline = true;
        self
    }

    /// Mark as providing explainability.
    pub fn with_explainability(mut self) -> Self {
        self.provides_explainability = true;
        self
    }

    /// Mark as handling evidence.
    pub fn with_evidence_handling(mut self) -> Self {
        self.handles_evidence = true;
        self
    }

    /// Set layers used.
    pub fn with_layers(mut self, layers: Vec<RetrievalLayer>) -> Self {
        self.layers_used = layers;
        self
    }
}

/// Result of skill contract validation.
#[derive(Debug, Clone)]
pub struct SkillContractValidation {
    /// Skill name
    pub skill_name: String,

    /// Whether the contract is valid
    pub is_valid: bool,

    /// List of issues found
    pub issues: Vec<SkillContractIssue>,
}

impl SkillContractValidation {
    /// Get errors only.
    pub fn errors(&self) -> Vec<&SkillContractIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect()
    }

    /// Get warnings only.
    pub fn warnings(&self) -> Vec<&SkillContractIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Warning)
            .collect()
    }

    /// Format as a report string.
    pub fn to_report(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Skill Contract Validation: {}", self.skill_name));
        lines.push(format!(
            "Status: {}",
            if self.is_valid { "VALID" } else { "INVALID" }
        ));
        lines.push(String::new());

        if !self.issues.is_empty() {
            lines.push("Issues:".to_string());
            for issue in &self.issues {
                let icon = match issue.severity {
                    IssueSeverity::Error => "ERROR",
                    IssueSeverity::Warning => "WARN",
                    IssueSeverity::Info => "INFO",
                };
                lines.push(format!(
                    "  [{}] {}: {}",
                    icon, issue.requirement, issue.message
                ));
            }
        } else {
            lines.push("No issues found.".to_string());
        }

        lines.join("\n")
    }
}

/// A single issue found during validation.
#[derive(Debug, Clone)]
pub struct SkillContractIssue {
    /// Which requirement was violated
    pub requirement: String,

    /// Severity of the issue
    pub severity: IssueSeverity,

    /// Description of the issue
    pub message: String,
}

/// Severity of a contract issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Must be fixed
    Error,
    /// Should be fixed
    Warning,
    /// Informational
    Info,
}

/// Generate SKILL.md content for a retrieval-capable skill.
///
/// Per PRD Section 8: SKILL.md Requirements
pub fn generate_skill_md_section(contract: &SkillContract) -> String {
    let mut lines = Vec::new();

    lines.push("## Memory Integration".to_string());
    lines.push(String::new());
    lines.push("### Retrieval Layers Used".to_string());

    let all_layers = [
        (RetrievalLayer::Topics, "Topics (optional)"),
        (RetrievalLayer::Vector, "Vector (optional)"),
        (RetrievalLayer::BM25, "BM25 (optional)"),
        (
            RetrievalLayer::Agentic,
            "Agentic TOC Search (always available)",
        ),
    ];

    for (layer, description) in all_layers {
        let checked = if contract.layers_used.contains(&layer) || layer == RetrievalLayer::Agentic {
            "[x]"
        } else {
            "[ ]"
        };
        lines.push(format!("- {} {}", checked, description));
    }

    lines.push(String::new());
    lines.push("### Fallback Behavior".to_string());
    lines.push(String::new());

    if contract.has_fallback_discipline {
        lines
            .push("This skill follows the fallback chain when layers are unavailable:".to_string());
        lines.push(String::new());
        for layer in &contract.layers_used {
            lines.push(format!("1. Try {} first", layer.as_str()));
        }
        lines.push("2. Fall back to Agentic TOC Search if all else fails".to_string());
        lines.push("3. Never hard-fail if agentic search can run".to_string());
    } else {
        lines.push("*Fallback behavior not documented*".to_string());
    }

    lines.push(String::new());
    lines.push("### Stop Conditions".to_string());
    lines.push(String::new());

    if let Some(ref conditions) = contract.custom_stop_conditions {
        lines.push(format!("- Max Depth: {}", conditions.max_depth));
        lines.push(format!("- Max Nodes: {}", conditions.max_nodes));
        lines.push(format!("- Timeout: {}ms", conditions.timeout_ms));
        lines.push(format!("- Beam Width: {}", conditions.beam_width));
    } else {
        lines.push("Uses default stop conditions.".to_string());
    }

    lines.push(String::new());
    lines.push("### Configuration".to_string());
    lines.push(String::new());
    lines.push("Layers can be enabled/disabled via configuration:".to_string());
    lines.push(String::new());
    lines.push("```toml".to_string());
    lines.push("[teleport]".to_string());
    lines.push("bm25.enabled = true".to_string());
    lines.push(String::new());
    lines.push("[vector]".to_string());
    lines.push("enabled = true".to_string());
    lines.push(String::new());
    lines.push("[topics]".to_string());
    lines.push("enabled = true".to_string());
    lines.push("```".to_string());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explainability_summary() {
        let payload = ExplainabilityPayload {
            intent: QueryIntent::Answer,
            tier: CapabilityTier::Hybrid,
            mode: ExecutionMode::Sequential,
            candidates_considered: vec![
                RetrievalLayer::Hybrid,
                RetrievalLayer::BM25,
                RetrievalLayer::Agentic,
            ],
            winner: RetrievalLayer::BM25,
            why_winner: "BM25 returned high confidence results".to_string(),
            fallback_occurred: false,
            fallback_reason: None,
            stop_conditions: StopConditions::default(),
            bounds_hit: vec![],
            total_time_ms: 150,
            result_count: 5,
            grip_ids: vec!["grip-1".to_string(), "grip-2".to_string()],
        };

        let summary = payload.to_summary();
        assert!(summary.contains("Hybrid"));
        assert!(summary.contains("bm25"));
        assert!(summary.contains("5"));
        assert!(summary.contains("150ms"));
        assert!(summary.contains("2 grips"));
    }

    #[test]
    fn test_explainability_markdown() {
        let payload = ExplainabilityPayload {
            intent: QueryIntent::Locate,
            tier: CapabilityTier::Full,
            mode: ExecutionMode::Sequential,
            candidates_considered: vec![RetrievalLayer::BM25, RetrievalLayer::Agentic],
            winner: RetrievalLayer::BM25,
            why_winner: "Exact match found".to_string(),
            fallback_occurred: false,
            fallback_reason: None,
            stop_conditions: StopConditions::default(),
            bounds_hit: vec![],
            total_time_ms: 50,
            result_count: 1,
            grip_ids: vec![],
        };

        let md = payload.to_markdown();
        assert!(md.contains("## Retrieval Method"));
        assert!(md.contains("**Tier:**"));
        assert!(md.contains("locate"));
    }

    #[test]
    fn test_skill_contract_valid() {
        let contract = SkillContract::new("memory-query")
            .with_capability_detection()
            .with_budget_enforcement()
            .with_fallback_discipline()
            .with_explainability()
            .with_evidence_handling()
            .with_layers(vec![
                RetrievalLayer::BM25,
                RetrievalLayer::Vector,
                RetrievalLayer::Agentic,
            ]);

        let validation = contract.validate();
        assert!(validation.is_valid);
        assert!(validation.errors().is_empty());
    }

    #[test]
    fn test_skill_contract_invalid() {
        let contract = SkillContract::new("bad-skill");

        let validation = contract.validate();
        assert!(!validation.is_valid);
        assert!(!validation.errors().is_empty());
    }

    #[test]
    fn test_skill_contract_warnings() {
        let contract = SkillContract::new("partial-skill")
            .with_capability_detection()
            .with_budget_enforcement()
            .with_fallback_discipline();

        let validation = contract.validate();
        assert!(validation.is_valid); // Still valid, just has warnings
        assert!(!validation.warnings().is_empty());
    }

    #[test]
    fn test_generate_skill_md() {
        let contract = SkillContract::new("memory-query")
            .with_capability_detection()
            .with_budget_enforcement()
            .with_fallback_discipline()
            .with_layers(vec![RetrievalLayer::BM25, RetrievalLayer::Vector]);

        let md = generate_skill_md_section(&contract);
        assert!(md.contains("## Memory Integration"));
        assert!(md.contains("### Retrieval Layers Used"));
        assert!(md.contains("[x] BM25"));
        assert!(md.contains("[x] Vector"));
        assert!(md.contains("[x] Agentic TOC Search"));
    }

    #[test]
    fn test_validation_report() {
        let contract = SkillContract::new("test-skill").with_capability_detection();

        let validation = contract.validate();
        let report = validation.to_report();

        assert!(report.contains("test-skill"));
        assert!(report.contains("INVALID"));
        assert!(report.contains("ERROR"));
    }
}
