//! Committed paraphrase families for the mock vector layer (QUAL-01).
//!
//! If a query or document contains any phrase in a family, the mock vector
//! path appends every member of that family before TF-IDF cosine. This is a
//! fixture-level semantic stand-in, not Candle/HNSW.

/// Each inner slice is one paraphrase family. Matching is case-insensitive
/// substring on the full phrase, not on individual tokens.
pub const FAMILIES: &[&[&str]] = &[
    &[
        "token expiry policy",
        "jwt lifetime",
        "fifteen minutes",
        "rotating refresh credentials",
    ],
    &[
        "container orchestration cutover",
        "eks migration",
        "karpenter node provisioning",
        "12 february 2026",
    ],
    &[
        "distributed tracing vendor",
        "opentelemetry",
        "grafana tempo",
        "5% sample rate",
    ],
    &[
        "feature toggle saas",
        "unleash is self-hosted",
        "rejected launchdarkly",
        "unleash",
    ],
    &[
        "background job persistence",
        "skip locked",
        "postgres skip locked",
        "redis lists",
    ],
    &[
        "gateway throttle quota",
        "120 requests per minute",
        "20/min",
        "partner credentials",
    ],
    &[
        "session cache duration",
        "fifteen-minute default ttl",
        "redis look-aside",
        "look-aside",
    ],
    &[
        "primary pager rotation",
        "avery takes first on-call",
        "friday 16:00",
        "week of 3 march",
    ],
    &[
        "null avatar crash",
        "empty option",
        "default_photo_url",
        "profile handler panicked",
    ],
    &["log shipping backend", "promtail", "loki", "fluent bit"],
    &[
        "schema migration utility",
        "atlas apply",
        "expand-contract",
        "postgres tables",
    ],
    &[
        "secret storage backend",
        "sops plus age encryption",
        "age encryption",
        "vault is out",
    ],
    &[
        "blue green release",
        "argo rollouts",
        "canary",
        "abandoned full swaps",
    ],
    &[
        "search ranking algorithm",
        "first-pass bm25",
        "cross-encoder rerank",
        "bm25 then a cross-encoder",
    ],
    &[
        "object storage lifecycle",
        "intelligent-tiering",
        "glacier after 30 days",
        "s3 intelligent-tiering",
    ],
    &[
        "identity provider cutover",
        "keycloak replaces auth0",
        "saml mappings stay",
        "keycloak",
    ],
];

/// Append every member of any family whose phrase appears in `text`.
pub fn expand(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut extra = Vec::new();
    for family in FAMILIES {
        if family.iter().any(|p| lower.contains(&p.to_lowercase())) {
            extra.extend(family.iter().copied());
        }
    }
    if extra.is_empty() {
        text.to_string()
    } else {
        format!("{text} {}", extra.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_adds_family_on_query_phrase() {
        let out = expand("token expiry policy");
        let lower = out.to_lowercase();
        assert!(lower.contains("jwt lifetime"), "{out}");
        assert!(lower.contains("fifteen minutes"), "{out}");
    }

    #[test]
    fn expand_adds_family_on_relevant_phrase() {
        let out =
            expand("We set JWT lifetime to fifteen minutes with rotating refresh credentials.");
        let lower = out.to_lowercase();
        assert!(lower.contains("token expiry policy"), "{out}");
    }

    #[test]
    fn expand_does_not_fire_on_separated_query_words() {
        let out = expand(
            "The cafeteria token of appreciation, and the HR policy, have an expiry of one year.",
        );
        assert_eq!(out, expand("The cafeteria token of appreciation, and the HR policy, have an expiry of one year."));
        assert!(
            !out.to_lowercase().contains("jwt lifetime"),
            "distractor must not expand: {out}"
        );
    }
}
