use std::fmt::Write;

/// Convert a `serde_json::Value` object to a simple YAML string.
///
/// Handles: strings, numbers, booleans, arrays, nested objects.
/// Returns empty string for non-object input.
pub fn value_to_yaml(value: &serde_json::Value) -> String {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return String::new(),
    };
    let mut out = String::new();
    for (key, val) in obj {
        write_yaml_value(&mut out, key, val, 0);
    }
    out
}

/// Reconstruct a markdown file from YAML frontmatter and body content.
///
/// If frontmatter produces empty YAML, returns just the body.
/// Otherwise returns `---\n{yaml}---\n\n{body}`.
pub fn reconstruct_md(frontmatter: &serde_json::Value, body: &str) -> String {
    let yaml = value_to_yaml(frontmatter);
    if yaml.is_empty() {
        body.to_string()
    } else {
        format!("---\n{yaml}---\n\n{body}")
    }
}

/// Replace all occurrences of `from` with `to` in the content string.
pub fn rewrite_paths(content: &str, from: &str, to: &str) -> String {
    content.replace(from, to)
}

/// Write a single YAML key-value pair at the given indentation level.
fn write_yaml_value(out: &mut String, key: &str, val: &serde_json::Value, indent: usize) {
    let prefix = " ".repeat(indent);
    match val {
        serde_json::Value::String(s) => {
            if s.contains('\n') {
                // Block scalar for multi-line strings
                let _ = writeln!(out, "{prefix}{key}: |");
                let inner_prefix = " ".repeat(indent + 2);
                for line in s.lines() {
                    let _ = writeln!(out, "{inner_prefix}{line}");
                }
            } else if needs_quoting(s) {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                let _ = writeln!(out, "{prefix}{key}: \"{escaped}\"");
            } else {
                let _ = writeln!(out, "{prefix}{key}: {s}");
            }
        }
        serde_json::Value::Number(n) => {
            let _ = writeln!(out, "{prefix}{key}: {n}");
        }
        serde_json::Value::Bool(b) => {
            let _ = writeln!(out, "{prefix}{key}: {b}");
        }
        serde_json::Value::Array(arr) => {
            let _ = writeln!(out, "{prefix}{key}:");
            for item in arr {
                match item {
                    serde_json::Value::String(s) => {
                        if needs_quoting(s) {
                            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                            let _ = writeln!(out, "{prefix}  - \"{escaped}\"");
                        } else {
                            let _ = writeln!(out, "{prefix}  - {s}");
                        }
                    }
                    serde_json::Value::Number(n) => {
                        let _ = writeln!(out, "{prefix}  - {n}");
                    }
                    serde_json::Value::Bool(b) => {
                        let _ = writeln!(out, "{prefix}  - {b}");
                    }
                    serde_json::Value::Object(map) => {
                        // First entry on the `- ` line, rest indented
                        let mut first = true;
                        for (k, v) in map {
                            if first {
                                let _ = write!(out, "{prefix}  - ");
                                write_yaml_inline(out, k, v);
                                let _ = writeln!(out);
                                first = false;
                            } else {
                                let _ = write!(out, "{prefix}    ");
                                write_yaml_inline(out, k, v);
                                let _ = writeln!(out);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        serde_json::Value::Object(map) => {
            let _ = writeln!(out, "{prefix}{key}:");
            for (k, v) in map {
                write_yaml_value(out, k, v, indent + 2);
            }
        }
        serde_json::Value::Null => {
            let _ = writeln!(out, "{prefix}{key}: null");
        }
    }
}

/// Write inline key: value (for simple values within arrays of objects).
fn write_yaml_inline(out: &mut String, key: &str, val: &serde_json::Value) {
    match val {
        serde_json::Value::String(s) => {
            if needs_quoting(s) {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                let _ = write!(out, "{key}: \"{escaped}\"");
            } else {
                let _ = write!(out, "{key}: {s}");
            }
        }
        serde_json::Value::Number(n) => {
            let _ = write!(out, "{key}: {n}");
        }
        serde_json::Value::Bool(b) => {
            let _ = write!(out, "{key}: {b}");
        }
        _ => {
            let _ = write!(out, "{key}: {val}");
        }
    }
}

/// Check if a YAML string value needs quoting.
fn needs_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    // Quote if contains `: `, `#`, or starts with special YAML chars
    if s.contains(": ") || s.contains('#') {
        return true;
    }
    let first = s.as_bytes()[0];
    // Special YAML characters that need quoting at start
    matches!(
        first,
        b'*' | b'&'
            | b'!'
            | b'{'
            | b'}'
            | b'['
            | b']'
            | b','
            | b'?'
            | b'-'
            | b'@'
            | b'`'
            | b'\''
            | b'"'
            | b'%'
            | b'|'
            | b'>'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn value_to_yaml_simple_string() {
        let val = json!({"name": "test-command"});
        let yaml = value_to_yaml(&val);
        assert_eq!(yaml, "name: test-command\n");
    }

    #[test]
    fn value_to_yaml_number() {
        let val = json!({"count": 42});
        let yaml = value_to_yaml(&val);
        assert_eq!(yaml, "count: 42\n");
    }

    #[test]
    fn value_to_yaml_boolean() {
        let val = json!({"enabled": true});
        let yaml = value_to_yaml(&val);
        assert_eq!(yaml, "enabled: true\n");
    }

    #[test]
    fn value_to_yaml_array() {
        let val = json!({"items": ["alpha", "beta"]});
        let yaml = value_to_yaml(&val);
        assert_eq!(yaml, "items:\n  - alpha\n  - beta\n");
    }

    #[test]
    fn value_to_yaml_nested_object() {
        let val = json!({"outer": {"inner": "value"}});
        let yaml = value_to_yaml(&val);
        assert_eq!(yaml, "outer:\n  inner: value\n");
    }

    #[test]
    fn value_to_yaml_non_object_returns_empty() {
        assert_eq!(value_to_yaml(&json!("string")), "");
        assert_eq!(value_to_yaml(&json!(42)), "");
        assert_eq!(value_to_yaml(&json!(null)), "");
    }

    #[test]
    fn value_to_yaml_quotes_string_with_colon_space() {
        let val = json!({"description": "search: conversations"});
        let yaml = value_to_yaml(&val);
        assert_eq!(yaml, "description: \"search: conversations\"\n");
    }

    #[test]
    fn value_to_yaml_quotes_string_with_hash() {
        let val = json!({"color": "#0000FF"});
        let yaml = value_to_yaml(&val);
        assert_eq!(yaml, "color: \"#0000FF\"\n");
    }

    #[test]
    fn value_to_yaml_block_scalar_for_multiline() {
        let val = json!({"description": "line one\nline two"});
        let yaml = value_to_yaml(&val);
        assert_eq!(yaml, "description: |\n  line one\n  line two\n");
    }

    #[test]
    fn reconstruct_md_with_frontmatter() {
        let fm = json!({"name": "test"});
        let body = "Hello world";
        let result = reconstruct_md(&fm, body);
        assert_eq!(result, "---\nname: test\n---\n\nHello world");
    }

    #[test]
    fn reconstruct_md_without_frontmatter() {
        let fm = json!(null);
        let body = "Just body content";
        let result = reconstruct_md(&fm, body);
        assert_eq!(result, "Just body content");
    }

    #[test]
    fn reconstruct_md_empty_object_frontmatter() {
        let fm = json!({});
        let body = "Body here";
        let result = reconstruct_md(&fm, body);
        // Empty object produces empty yaml string
        assert_eq!(result, "Body here");
    }

    #[test]
    fn rewrite_paths_basic() {
        let content = "Load from ~/.claude/skills/foo";
        let result = rewrite_paths(content, "~/.claude/", "~/.config/agent-memory/");
        assert_eq!(result, "Load from ~/.config/agent-memory/skills/foo");
    }

    #[test]
    fn rewrite_paths_multiple_occurrences() {
        let content = "A ~/.claude/x and ~/.claude/y";
        let result = rewrite_paths(content, "~/.claude/", "~/.config/agent-memory/");
        assert_eq!(
            result,
            "A ~/.config/agent-memory/x and ~/.config/agent-memory/y"
        );
    }

    #[test]
    fn rewrite_paths_no_match_unchanged() {
        let content = "No paths here";
        let result = rewrite_paths(content, "~/.claude/", "~/.config/agent-memory/");
        assert_eq!(result, "No paths here");
    }
}
