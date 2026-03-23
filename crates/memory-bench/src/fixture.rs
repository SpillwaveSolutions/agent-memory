use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A collection of test cases loaded from a TOML fixture file.
#[derive(Debug, Deserialize, Serialize)]
pub struct Fixture {
    #[serde(rename = "test")]
    pub tests: Vec<TestCase>,
}

/// A single benchmark test case.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestCase {
    /// Unique identifier for this test case.
    pub id: String,
    /// Human-readable description of what the test verifies.
    pub description: String,
    /// Paths to JSONL session files to ingest before running the query.
    pub setup: Vec<String>,
    /// The query to run against the memory system.
    pub query: String,
    /// Case-insensitive substrings that the response should contain.
    pub expected_contains: Vec<String>,
    /// Maximum token budget for the response.
    pub max_tokens: usize,
}

impl Fixture {
    /// Load and validate a fixture from a TOML file.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let fixture: Fixture = toml::from_str(&content)?;

        for tc in &fixture.tests {
            if tc.id.is_empty() {
                bail!("Test case has empty id in {}", path.display());
            }
            if tc.query.is_empty() {
                bail!(
                    "Test case '{}' has empty query in {}",
                    tc.id,
                    path.display()
                );
            }
        }

        Ok(fixture)
    }

    /// Load all `.toml` fixture files from a directory and collect their test cases.
    pub fn load_dir(dir: &Path) -> Result<Vec<TestCase>> {
        let mut all_tests = Vec::new();

        let mut entries: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "toml"))
            .collect();

        entries.sort_by_key(|e| e.path());

        for entry in entries {
            let fixture = Self::load(&entry.path())?;
            all_tests.extend(fixture.tests);
        }

        Ok(all_tests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_fixture_parses_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            r#"
[[test]]
id = "t-001"
description = "recall a decision"
setup = ["sessions/auth.jsonl"]
query = "what auth did we pick?"
expected_contains = ["JWT"]
max_tokens = 500

[[test]]
id = "t-002"
description = "recall a bug fix"
setup = ["sessions/bug.jsonl"]
query = "how was the bug fixed?"
expected_contains = ["Option"]
max_tokens = 400
"#
        )
        .unwrap();

        let fixture = Fixture::load(&path).unwrap();
        assert_eq!(fixture.tests.len(), 2);
        assert_eq!(fixture.tests[0].id, "t-001");
        assert_eq!(fixture.tests[1].id, "t-002");
        assert_eq!(fixture.tests[0].max_tokens, 500);
    }

    #[test]
    fn test_fixture_validates_empty_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            r#"
[[test]]
id = ""
description = "bad test"
setup = []
query = "something"
expected_contains = []
max_tokens = 100
"#
        )
        .unwrap();

        let result = Fixture::load(&path);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("empty id"),
            "Error should mention empty id"
        );
    }

    #[test]
    fn test_fixture_validates_empty_query() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            r#"
[[test]]
id = "t-001"
description = "bad test"
setup = []
query = ""
expected_contains = []
max_tokens = 100
"#
        )
        .unwrap();

        let result = Fixture::load(&path);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("empty query"),
            "Error should mention empty query"
        );
    }

    #[test]
    fn test_load_dir_collects_all_fixtures() {
        let dir = tempfile::tempdir().unwrap();

        // Create first fixture file
        let path1 = dir.path().join("a.toml");
        let mut f1 = std::fs::File::create(&path1).unwrap();
        write!(
            f1,
            r#"
[[test]]
id = "a-001"
description = "test a"
setup = []
query = "query a"
expected_contains = []
max_tokens = 100
"#
        )
        .unwrap();

        // Create second fixture file
        let path2 = dir.path().join("b.toml");
        let mut f2 = std::fs::File::create(&path2).unwrap();
        write!(
            f2,
            r#"
[[test]]
id = "b-001"
description = "test b1"
setup = []
query = "query b1"
expected_contains = []
max_tokens = 200

[[test]]
id = "b-002"
description = "test b2"
setup = []
query = "query b2"
expected_contains = []
max_tokens = 300
"#
        )
        .unwrap();

        let tests = Fixture::load_dir(dir.path()).unwrap();
        assert_eq!(tests.len(), 3);
        assert_eq!(tests[0].id, "a-001");
        assert_eq!(tests[1].id, "b-001");
        assert_eq!(tests[2].id, "b-002");
    }
}
