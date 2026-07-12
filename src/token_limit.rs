/// Response truncation — 40 KB cap (~10K tokens).
///
/// Any single MCP tool response that exceeds this limit is truncated with a
/// clear hint so agents know to narrow their query.
pub const MAX_RESPONSE_BYTES: usize = 40_000;

/// Truncate `text` to at most [`MAX_RESPONSE_BYTES`] bytes, appending a hint
/// when truncation occurs. Always returns valid UTF-8 (truncates at a char
/// boundary, not a raw byte index).
#[must_use]
pub fn truncate_response(text: &str) -> String {
    if text.len() <= MAX_RESPONSE_BYTES {
        return text.to_string();
    }
    // Walk back to the last valid char boundary at or before the cap.
    let mut end = MAX_RESPONSE_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n\n[TRUNCATED: response exceeded 40 KB (~10K token) limit. \
         Use limit/offset parameters or more specific filters to reduce output size.]",
        &text[..end]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_returned_unchanged() {
        let s = "hello world";
        assert_eq!(truncate_response(s), s);
    }

    #[test]
    fn long_text_is_truncated_with_hint() {
        let s = "x".repeat(MAX_RESPONSE_BYTES + 10_000);
        let result = truncate_response(&s);
        assert!(result.len() < s.len());
        assert!(result.contains("[TRUNCATED:"));
    }

    #[test]
    fn truncation_at_char_boundary() {
        // Multi-byte character right at the boundary should not panic.
        let mut s = "a".repeat(MAX_RESPONSE_BYTES - 1);
        s.push('€'); // 3-byte UTF-8
        s.push_str("extra");
        let result = truncate_response(&s);
        assert!(result.contains("[TRUNCATED:"));
    }
}
