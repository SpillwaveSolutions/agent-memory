//! File writer with dry-run support and managed-section markers.
//!
//! All converters produce `Vec<ConvertedFile>`. The [`write_files`] function
//! either writes them to disk or prints a dry-run report -- no per-converter
//! changes needed.
//!
//! Managed-section markers allow safe merge, upgrade, and uninstall of
//! content injected into shared config files.

use std::fmt;
use std::path::Path;

use anyhow::{Context, Result};

use crate::types::ConvertedFile;

// Re-export marker constants for convenience.
pub use crate::types::{MANAGED_BEGIN, MANAGED_END, MANAGED_JSON_KEY, MANAGED_JSON_VALUE};

/// Summary of a write operation.
#[derive(Debug, Clone, Default)]
pub struct WriteReport {
    /// Number of newly created files.
    pub created: usize,
    /// Number of overwritten files.
    pub overwritten: usize,
    /// Number of skipped files (unused for now, reserved for future filters).
    pub skipped: usize,
}

impl fmt::Display for WriteReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} created, {} overwritten, {} skipped",
            self.created, self.overwritten, self.skipped
        )
    }
}

/// Write converted files to disk, or print a dry-run report.
///
/// When `dry_run` is `true`, prints `[DRY-RUN] CREATE` or `[DRY-RUN] OVERWRITE`
/// for each file without modifying the filesystem.
///
/// When `dry_run` is `false`, creates parent directories and writes each file.
pub fn write_files(files: &[ConvertedFile], dry_run: bool) -> Result<WriteReport> {
    let mut report = WriteReport::default();

    for file in files {
        let exists = file.target_path.exists();

        if dry_run {
            let action = if exists { "OVERWRITE" } else { "CREATE" };
            println!("[DRY-RUN] {} {}", action, file.target_path.display());
            println!("  {} bytes", file.content.len());
        } else {
            if let Some(parent) = file.target_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating directory {}", parent.display()))?;
            }
            std::fs::write(&file.target_path, &file.content)
                .with_context(|| format!("writing {}", file.target_path.display()))?;
        }

        if exists {
            report.overwritten += 1;
        } else {
            report.created += 1;
        }
    }

    Ok(report)
}

/// Merge managed content into a file using begin/end markers.
///
/// Handles three cases:
/// 1. **File does not exist** -- creates a new file with markers wrapping content.
/// 2. **File has markers** -- replaces content between markers (inclusive).
/// 3. **File exists without markers** -- appends markers + content at end.
pub fn merge_managed_section(file_path: &Path, managed_content: &str) -> Result<()> {
    let managed_block = format!("{}\n{}\n{}\n", MANAGED_BEGIN, managed_content, MANAGED_END);

    if !file_path.exists() {
        // Case 1: new file
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }
        std::fs::write(file_path, &managed_block)
            .with_context(|| format!("writing {}", file_path.display()))?;
        return Ok(());
    }

    let existing = std::fs::read_to_string(file_path)
        .with_context(|| format!("reading {}", file_path.display()))?;

    if let (Some(start), Some(end_pos)) = (existing.find(MANAGED_BEGIN), existing.find(MANAGED_END))
    {
        // Case 2: file has markers -- replace between them (inclusive)
        let before = &existing[..start];
        let after = &existing[end_pos + MANAGED_END.len()..];
        let updated = format!("{}{}{}", before, managed_block, after);
        std::fs::write(file_path, updated)
            .with_context(|| format!("writing {}", file_path.display()))?;
    } else {
        // Case 3: file exists without markers -- append
        let updated = format!("{}\n{}", existing.trim_end(), managed_block);
        std::fs::write(file_path, updated)
            .with_context(|| format!("writing {}", file_path.display()))?;
    }

    Ok(())
}

/// Remove managed section from a file if present.
///
/// Returns `Ok(true)` if markers were found and removed, `Ok(false)` otherwise.
/// Supports future `--uninstall` functionality.
pub fn remove_managed_section(file_path: &Path) -> Result<bool> {
    if !file_path.exists() {
        return Ok(false);
    }

    let existing = std::fs::read_to_string(file_path)
        .with_context(|| format!("reading {}", file_path.display()))?;

    if let (Some(start), Some(end_pos)) = (existing.find(MANAGED_BEGIN), existing.find(MANAGED_END))
    {
        let before = &existing[..start];
        let after = &existing[end_pos + MANAGED_END.len()..];
        // Trim trailing whitespace from the join to avoid blank lines
        let updated = format!("{}{}", before.trim_end(), after);
        let updated = if updated.is_empty() {
            String::new()
        } else {
            format!("{}\n", updated.trim_end())
        };
        std::fs::write(file_path, updated)
            .with_context(|| format!("writing {}", file_path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn marker_constants_match_locked_format() {
        assert_eq!(
            MANAGED_BEGIN,
            "# --- MANAGED BY memory-installer (DO NOT EDIT) ---"
        );
        assert_eq!(MANAGED_END, "# --- END MANAGED ---");
    }

    #[test]
    fn write_files_dry_run_does_not_create_files() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("output.txt");
        let files = vec![ConvertedFile {
            target_path: target.clone(),
            content: "hello world".to_string(),
        }];

        let report = write_files(&files, true).unwrap();
        assert!(!target.exists(), "dry-run should not create files");
        assert_eq!(report.created, 1);
        assert_eq!(report.overwritten, 0);
    }

    #[test]
    fn write_files_creates_files_with_correct_content() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("output.txt");
        let files = vec![ConvertedFile {
            target_path: target.clone(),
            content: "hello world".to_string(),
        }];

        let report = write_files(&files, false).unwrap();
        assert!(target.exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello world");
        assert_eq!(report.created, 1);
    }

    #[test]
    fn write_files_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("nested/deep/output.txt");
        let files = vec![ConvertedFile {
            target_path: target.clone(),
            content: "nested content".to_string(),
        }];

        let report = write_files(&files, false).unwrap();
        assert!(target.exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "nested content");
        assert_eq!(report.created, 1);
    }

    #[test]
    fn write_files_tracks_overwrite() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("existing.txt");
        std::fs::write(&target, "old content").unwrap();

        let files = vec![ConvertedFile {
            target_path: target.clone(),
            content: "new content".to_string(),
        }];

        let report = write_files(&files, false).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new content");
        assert_eq!(report.overwritten, 1);
        assert_eq!(report.created, 0);
    }

    #[test]
    fn merge_managed_section_creates_new_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("config.txt");

        merge_managed_section(&file, "key = value").unwrap();

        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.contains(MANAGED_BEGIN));
        assert!(content.contains("key = value"));
        assert!(content.contains(MANAGED_END));
    }

    #[test]
    fn merge_managed_section_replaces_existing_markers() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("config.txt");

        // Write initial content with markers
        let initial = format!(
            "user line 1\n{}\nold managed content\n{}\nuser line 2\n",
            MANAGED_BEGIN, MANAGED_END
        );
        std::fs::write(&file, initial).unwrap();

        merge_managed_section(&file, "new managed content").unwrap();

        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.contains("user line 1"));
        assert!(content.contains("new managed content"));
        assert!(!content.contains("old managed content"));
        assert!(content.contains("user line 2"));
    }

    #[test]
    fn merge_managed_section_appends_to_file_without_markers() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("config.txt");

        std::fs::write(&file, "existing content\n").unwrap();

        merge_managed_section(&file, "appended content").unwrap();

        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.starts_with("existing content"));
        assert!(content.contains(MANAGED_BEGIN));
        assert!(content.contains("appended content"));
        assert!(content.contains(MANAGED_END));
    }

    #[test]
    fn remove_managed_section_returns_false_for_missing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("nonexistent.txt");
        assert!(!remove_managed_section(&file).unwrap());
    }

    #[test]
    fn remove_managed_section_returns_false_for_file_without_markers() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("config.txt");
        std::fs::write(&file, "just user content\n").unwrap();
        assert!(!remove_managed_section(&file).unwrap());
    }

    #[test]
    fn remove_managed_section_removes_markers_and_content() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("config.txt");

        let content = format!(
            "user before\n{}\nmanaged stuff\n{}\nuser after\n",
            MANAGED_BEGIN, MANAGED_END
        );
        std::fs::write(&file, content).unwrap();

        assert!(remove_managed_section(&file).unwrap());

        let remaining = std::fs::read_to_string(&file).unwrap();
        assert!(remaining.contains("user before"));
        assert!(remaining.contains("user after"));
        assert!(!remaining.contains(MANAGED_BEGIN));
        assert!(!remaining.contains("managed stuff"));
    }

    #[test]
    fn dry_run_reports_overwrite_for_existing_files() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("existing.txt");
        std::fs::write(&target, "old").unwrap();

        let files = vec![ConvertedFile {
            target_path: target.clone(),
            content: "new".to_string(),
        }];

        let report = write_files(&files, true).unwrap();
        assert!(target.exists());
        // File should still have old content (dry-run)
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
        assert_eq!(report.overwritten, 1);
        assert_eq!(report.created, 0);
    }

    #[test]
    fn merge_creates_parent_dirs_for_new_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("deep/nested/config.txt");

        merge_managed_section(&file, "content").unwrap();
        assert!(file.exists());
    }

    #[test]
    fn write_report_display() {
        let report = WriteReport {
            created: 3,
            overwritten: 1,
            skipped: 0,
        };
        assert_eq!(format!("{report}"), "3 created, 1 overwritten, 0 skipped");
    }
}
