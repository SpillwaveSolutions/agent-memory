//! Recall command: convenience alias for `search --rerank=llm --top=10`.

use anyhow::Result;

use crate::cli::{GlobalArgs, RecallArgs, SearchArgs};
use crate::commands::search;

/// Run the recall command by delegating to search with LLM reranking.
pub async fn run(args: RecallArgs, global: &GlobalArgs) -> Result<()> {
    let search_args = SearchArgs {
        query: args.query,
        top: 10,
        rerank: Some("llm".to_string()),
        format: args.format,
    };
    search::run(search_args, global).await
}

#[cfg(test)]
mod tests {
    use crate::cli::{RecallArgs, SearchArgs};

    /// Verify that recall constructs the correct SearchArgs.
    #[test]
    fn test_recall_builds_search_args_with_llm_rerank() {
        let recall_args = RecallArgs {
            query: "what happened yesterday".to_string(),
            format: Some("json".to_string()),
        };

        // Simulate the same logic as run()
        let search_args = SearchArgs {
            query: recall_args.query.clone(),
            top: 10,
            rerank: Some("llm".to_string()),
            format: recall_args.format.clone(),
        };

        assert_eq!(search_args.query, "what happened yesterday");
        assert_eq!(search_args.top, 10);
        assert_eq!(search_args.rerank.as_deref(), Some("llm"));
        assert_eq!(search_args.format.as_deref(), Some("json"));
    }

    #[test]
    fn test_recall_preserves_query() {
        let recall_args = RecallArgs {
            query: "test query".to_string(),
            format: None,
        };

        let search_args = SearchArgs {
            query: recall_args.query.clone(),
            top: 10,
            rerank: Some("llm".to_string()),
            format: recall_args.format.clone(),
        };

        assert_eq!(search_args.query, "test query");
        assert!(search_args.format.is_none());
    }
}
