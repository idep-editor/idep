use anyhow::Result;
use idep_core::buffer::Buffer;
use lsp_types::CompletionResponse;

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

    // Rank items (shorter labels first, then lexicographic)
    let ranked = crate::client::LspClient::rank_completions(items);

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
    fn applies_top_ranked_completion_to_buffer() {
        let mut buffer = Buffer::with_text("hello");
        buffer.move_cursor_to_end();

        let items = vec![
            CompletionItem {
                label: "world".into(),
                insert_text: Some("world".into()),
                ..Default::default()
            },
            CompletionItem {
                label: "foo".into(),
                insert_text: Some("foo".into()),
                ..Default::default()
            },
        ];

        let response = CompletionResponse::Array(items);
        apply_completions_to_buffer(&mut buffer, response).expect("apply");

        // "foo" is shorter, so it should be ranked first and applied
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
