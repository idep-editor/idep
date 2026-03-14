use anyhow::Result;
use idep_core::buffer::Buffer;
use lsp_types::CompletionResponse;
use std::collections::BTreeMap;

/// Rank completions: sort by sort_text (server intent), then label length, then lexicographic.
/// Uses BTreeMap for deterministic deduplication by label.
pub fn rank_completions(items: Vec<lsp_types::CompletionItem>) -> Vec<lsp_types::CompletionItem> {
    // Deduplicate by label (deterministic: BTreeMap keeps first insertion per key)
    let mut dedup: BTreeMap<String, lsp_types::CompletionItem> = BTreeMap::new();
    for item in items {
        dedup.entry(item.label.clone()).or_insert(item);
    }

    let mut ranked: Vec<_> = dedup.into_values().collect();
    ranked.sort_by(|a, b| {
        // Primary: sort_text (server-controlled ordering)
        let sort_a = a.sort_text.as_deref().unwrap_or(&a.label);
        let sort_b = b.sort_text.as_deref().unwrap_or(&b.label);
        sort_a
            .cmp(sort_b)
            // Secondary: label length (prefer shorter)
            .then_with(|| a.label.len().cmp(&b.label.len()))
            // Tertiary: lexicographic
            .then_with(|| a.label.cmp(&b.label))
    });
    ranked
}

/// Bridge LSP completion results into an idep-core buffer.
/// Ranks items, applies the top result to the buffer at cursor position.
pub fn apply_completions_to_buffer(
    buffer: &mut Buffer,
    response: CompletionResponse,
) -> Result<()> {
    let items = match response {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    };

    if items.is_empty() {
        return Ok(());
    }

    // Rank items (sort_text first, then label length, then lexicographic)
    let ranked = rank_completions(items);

    // Apply the top-ranked completion to the buffer
    if let Some(top) = ranked.first() {
        buffer.apply_completion(top);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::CompletionItem;

    #[test]
    fn rank_completions_sorts_by_sort_text() {
        let items = vec![
            CompletionItem {
                label: "beta".into(),
                sort_text: Some("2".into()),
                ..Default::default()
            },
            CompletionItem {
                label: "alpha".into(),
                sort_text: Some("1".into()),
                ..Default::default()
            },
        ];

        let ranked = rank_completions(items);
        let labels: Vec<_> = ranked.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["alpha", "beta"]);
    }

    #[test]
    fn rank_completions_dedup_deterministic() {
        let items = vec![
            CompletionItem {
                label: "foo".into(),
                sort_text: Some("1".into()),
                ..Default::default()
            },
            CompletionItem {
                label: "foo".into(),
                sort_text: Some("2".into()),
                ..Default::default()
            },
        ];

        let ranked = rank_completions(items);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].sort_text.as_deref(), Some("1"));
    }

    #[test]
    fn applies_top_ranked_completion_to_buffer() {
        let mut buffer = Buffer::with_text("hello");
        buffer.move_cursor_to_end();

        let items = vec![
            CompletionItem {
                label: "world".into(),
                insert_text: Some("world".into()),
                sort_text: Some("2".into()),
                ..Default::default()
            },
            CompletionItem {
                label: "foo".into(),
                insert_text: Some("foo".into()),
                sort_text: Some("1".into()),
                ..Default::default()
            },
        ];

        let response = CompletionResponse::Array(items);
        apply_completions_to_buffer(&mut buffer, response).expect("apply");

        // "foo" has sort_text "1", so it should be ranked first and applied
        assert_eq!(buffer.to_string(), "hellofoo");
    }

    #[test]
    fn handles_empty_completion_list() {
        let mut buffer = Buffer::with_text("test");
        let response = CompletionResponse::Array(vec![]);
        apply_completions_to_buffer(&mut buffer, response).expect("apply");
        assert_eq!(buffer.to_string(), "test");
    }
}
