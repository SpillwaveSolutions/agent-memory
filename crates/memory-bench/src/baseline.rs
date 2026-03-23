use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

/// Competitor baseline scores loaded from TOML.
#[derive(Debug, Deserialize)]
pub struct Baselines {
    pub memmachine: Option<CompetitorScore>,
    pub mem0: Option<CompetitorScore>,
}

/// Scores for a single competitor.
#[derive(Debug, Deserialize)]
pub struct CompetitorScore {
    pub locomo_score: Option<f64>,
    pub token_reduction: Option<f64>,
    pub latency_improvement: Option<f64>,
    pub accuracy_vs_openai_memory: Option<f64>,
    pub latency_reduction: Option<f64>,
}

impl Baselines {
    /// Load competitor baselines from a TOML file.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baselines_load() {
        let toml_content = r#"
[memmachine]
locomo_score = 0.91
token_reduction = 0.80
latency_improvement = 0.75

[mem0]
accuracy_vs_openai_memory = 0.26
token_reduction = 0.90
latency_reduction = 0.91
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("baselines.toml");
        std::fs::write(&path, toml_content).unwrap();

        let baselines = Baselines::load(&path).unwrap();

        let mm = baselines.memmachine.unwrap();
        assert_eq!(mm.locomo_score, Some(0.91));
        assert_eq!(mm.token_reduction, Some(0.80));
        assert_eq!(mm.latency_improvement, Some(0.75));

        let m0 = baselines.mem0.unwrap();
        assert_eq!(m0.accuracy_vs_openai_memory, Some(0.26));
        assert_eq!(m0.token_reduction, Some(0.90));
        assert_eq!(m0.latency_reduction, Some(0.91));
    }
}
