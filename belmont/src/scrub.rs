/// A streaming scrubber that replaces secret values with `belmont://NAME` references.
///
/// Maintains a boundary buffer to handle secrets split across chunk boundaries.
/// Values are sorted longest-first so a value that is a substring of another
/// gets replaced correctly.
pub struct Scrubber {
    /// Secret entries sorted by value length (longest first).
    entries: Vec<(String, String)>,
    /// Trailing bytes from the previous `feed()` call that might contain
    /// the start of a secret value spanning a chunk boundary.
    buffer: String,
    /// Length of the longest secret value — determines boundary buffer size.
    max_len: usize,
}

impl Scrubber {
    /// Create a new scrubber from name/value pairs.
    /// Empty values are filtered out. Values are sorted longest-first.
    pub fn new(entries: Vec<(String, String)>) -> Self {
        let mut entries: Vec<(String, String)> = entries
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .collect();
        entries.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        let max_len = entries.first().map(|(_, v)| v.len()).unwrap_or(0);
        Scrubber {
            entries,
            buffer: String::new(),
            max_len,
        }
    }

    /// Feed a chunk of output. Returns the safely-scrubbed prefix.
    /// Retains up to `max_len` bytes in the boundary buffer.
    pub fn feed(&mut self, chunk: &str) -> String {
        if self.entries.is_empty() {
            return chunk.to_string();
        }

        self.buffer.push_str(chunk);

        if self.buffer.len() <= self.max_len {
            // Not enough data to guarantee no secret spans the boundary.
            return String::new();
        }

        // Everything before the last max_len bytes is safe to emit.
        let emit_len = self.buffer.len() - self.max_len;
        let scrubbed_full = self.scrub_text(&self.buffer);
        let remaining_original = self.buffer[emit_len..].to_string();
        self.buffer = remaining_original;

        // We need to emit the scrubbed version of the prefix.
        // Scrub prefix and tail separately won't work (secret could span).
        // Scrub the whole thing, then figure out where to cut.
        //
        // The correct approach: scrub the full buffer. The emittable portion
        // is everything that maps to original bytes before emit_len.
        // Since scrubbing can change string lengths, we track by re-scrubbing
        // the remaining buffer and subtracting.
        let scrubbed_remaining = self.scrub_text(&self.buffer);

        // The emitted portion is: scrubbed_full with scrubbed_remaining stripped
        // from the end. This works because the tail is still in self.buffer.
        if let Some(prefix) = scrubbed_full.strip_suffix(&scrubbed_remaining) {
            return prefix.to_string();
        }

        // Fallback: if stripping doesn't work cleanly (overlapping replacements
        // changed boundaries), emit the scrubbed full and clear the buffer.
        // This sacrifices the boundary guarantee but avoids data loss.
        self.buffer.clear();
        scrubbed_full
    }

    /// Flush remaining buffered bytes at EOF.
    pub fn flush(&mut self) -> String {
        let remaining = std::mem::take(&mut self.buffer);
        self.scrub_text(&remaining)
    }

    /// Replace all secret values with their `belmont://NAME` references.
    fn scrub_text(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (name, value) in &self.entries {
            result = result.replace(value, &format!("belmont://{name}"));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scrubber(entries: Vec<(&str, &str)>) -> Scrubber {
        Scrubber::new(
            entries
                .into_iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
        )
    }

    #[test]
    fn single_chunk_replaces_secret() {
        let mut s = make_scrubber(vec![("DB", "hunter2")]);
        let out = s.feed("connecting to hunter2 now");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "connecting to belmont://DB now");
    }

    #[test]
    fn multiple_occurrences() {
        let mut s = make_scrubber(vec![("KEY", "abc123")]);
        let out = s.feed("first abc123 second abc123 done");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "first belmont://KEY second belmont://KEY done");
    }

    #[test]
    fn secret_spanning_chunk_boundary() {
        let mut s = make_scrubber(vec![("SECRET", "boundary")]);
        let out1 = s.feed("before boun");
        let out2 = s.feed("dary after");
        let out3 = s.flush();
        let combined = format!("{out1}{out2}{out3}");
        assert_eq!(combined, "before belmont://SECRET after");
    }

    #[test]
    fn longer_value_replaced_first() {
        let mut s = make_scrubber(vec![("SHORT", "abc"), ("LONG", "abcdef")]);
        let out = s.feed("value is abcdef here");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "value is belmont://LONG here");
    }

    #[test]
    fn empty_values_filtered() {
        let s = make_scrubber(vec![("EMPTY", ""), ("REAL", "secret")]);
        assert_eq!(s.entries.len(), 1);
    }

    #[test]
    fn no_secrets_passthrough() {
        let mut s = make_scrubber(vec![]);
        let out = s.feed("hello world");
        assert_eq!(out, "hello world");
    }

    #[test]
    fn flush_emits_remaining() {
        let mut s = make_scrubber(vec![("X", "secret")]);
        let out1 = s.feed("sec");
        // Not enough data, buffered
        assert_eq!(out1, "");
        let out2 = s.flush();
        assert_eq!(out2, "sec");
    }

    #[test]
    fn multiple_secrets_in_one_chunk() {
        let mut s = make_scrubber(vec![("A", "alpha"), ("B", "beta")]);
        let out = s.feed("start alpha middle beta end");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "start belmont://A middle belmont://B end");
    }

    #[test]
    fn output_containing_belmont_reference_untouched() {
        let mut s = make_scrubber(vec![("X", "secret")]);
        let out = s.feed("already has belmont://X in it");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "already has belmont://X in it");
    }

    #[test]
    fn substring_secret_not_replaced_when_longer_matches() {
        // "pass" is a substring of "password", longer value wins
        let mut s = make_scrubber(vec![("PARTIAL", "pass"), ("FULL", "password")]);
        let out = s.feed("my password is here");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "my belmont://FULL is here");
    }
}
