use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation as _;

/// Display-only destination used while a Markdown link URL is incomplete.
pub(crate) const PENDING_LINK_URL: &str = "hearth:pending-link";

const ZERO_WIDTH_SPACE: char = '\u{200B}';

/// Controls how provider deltas are divided into display-sized frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamingTextPacerConfig {
    /// Delay recommended between display chunks.
    pub frame_interval: Duration,
    /// Number of frames over which the current backlog should catch up.
    pub catch_up_frames: usize,
    /// Minimum number of graphemes emitted in one frame.
    pub min_graphemes_per_frame: usize,
    /// Maximum number of graphemes emitted in one frame.
    pub max_graphemes_per_frame: usize,
}

impl Default for StreamingTextPacerConfig {
    fn default() -> Self {
        Self {
            frame_interval: Duration::from_millis(24),
            catch_up_frames: 18,
            min_graphemes_per_frame: 12,
            max_graphemes_per_frame: 256,
        }
    }
}

/// Buffers text deltas and emits stable-paced chunks on grapheme boundaries
/// recognized within the currently buffered text.
///
/// This type owns no task or timer. The host drives it using [`Self::take_chunk`]
/// and waits for [`Self::frame_interval`] between non-empty chunks.
/// Provider deltas may still split one grapheme across updates; concatenating
/// all emitted chunks and [`Self::drain`] always preserves the original order.
#[derive(Clone, Debug, Default)]
pub struct StreamingTextPacer {
    pending: String,
    pending_graphemes: usize,
    config: StreamingTextPacerConfig,
}

impl StreamingTextPacer {
    /// Create a pacer using the default LLM streaming parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a pacer with explicit frame and backlog parameters.
    pub fn with_config(config: StreamingTextPacerConfig) -> Self {
        assert!(
            config.catch_up_frames > 0,
            "catch_up_frames must be positive"
        );
        assert!(
            config.min_graphemes_per_frame > 0,
            "min_graphemes_per_frame must be positive"
        );
        assert!(
            config.max_graphemes_per_frame >= config.min_graphemes_per_frame,
            "max_graphemes_per_frame must not be smaller than the minimum"
        );
        Self {
            pending: String::new(),
            pending_graphemes: 0,
            config,
        }
    }

    /// Append one provider delta without changing its order.
    pub fn push_str(&mut self, delta: &str) {
        self.pending.push_str(delta);
        self.pending_graphemes = self
            .pending_graphemes
            .saturating_add(delta.graphemes(true).count());
    }

    /// Return whether all buffered text has been emitted.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Return the recommended delay between calls to [`Self::take_chunk`].
    pub fn frame_interval(&self) -> Duration {
        self.config.frame_interval
    }

    /// Remove one adaptive display chunk without splitting a grapheme that is
    /// already present in the current buffer.
    ///
    /// A newline ends the chunk early so block boundaries can reach the
    /// Markdown parser without being combined with the following line.
    pub fn take_chunk(&mut self) -> Option<String> {
        if self.pending.is_empty() {
            return None;
        }

        let budget = self
            .pending_graphemes
            .div_ceil(self.config.catch_up_frames)
            .clamp(
                self.config.min_graphemes_per_frame,
                self.config.max_graphemes_per_frame,
            )
            .min(self.pending_graphemes);

        let mut count = 0;
        let mut end = self.pending.len();
        for (start, grapheme) in self.pending.grapheme_indices(true) {
            count += 1;
            end = start + grapheme.len();
            if grapheme == "\n" || count == budget {
                break;
            }
        }

        let remainder = self.pending.split_off(end);
        self.pending_graphemes = self.pending_graphemes.saturating_sub(count);
        let chunk = std::mem::replace(&mut self.pending, remainder);
        if self.pending.is_empty() {
            self.pending_graphemes = 0;
        }
        Some(chunk)
    }

    /// Remove all remaining text, preserving provider order.
    pub fn drain(&mut self) -> String {
        self.pending_graphemes = 0;
        std::mem::take(&mut self.pending)
    }
}

#[derive(Debug)]
struct OpenDelimiter {
    marker: char,
    owed: usize,
    opened_at: usize,
}

#[derive(Clone, Copy, Debug)]
struct OpenBracket {
    index: usize,
    image: bool,
}

#[derive(Clone, Copy, Debug)]
struct PendingDestination {
    close_bracket: usize,
    image: bool,
}

/// A display-only Markdown source plus metadata for synthetic visible text.
#[derive(Debug, PartialEq)]
pub(crate) struct MarkdownDisplayRepair {
    pub(crate) markdown: String,
    pub(crate) synthetic_text_suffix: Option<char>,
}

/// Close incomplete inline Markdown markers for a display-only tail parse.
pub(crate) fn close_hanging_markdown(text: &str) -> Option<MarkdownDisplayRepair> {
    let chars = text.char_indices().collect::<Vec<_>>();
    let at = |index: usize| chars.get(index).map(|&(_, ch)| ch);
    let mut delimiters: Vec<OpenDelimiter> = Vec::new();
    let mut brackets = Vec::new();
    let mut code: Option<(usize, usize)> = None;
    let mut last_content = None;
    let mut pending_url = None;
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index].1;
        if code.is_none() && ch == '\\' {
            if index + 1 < chars.len() {
                last_content = Some(index + 1);
            }
            index += 2;
            continue;
        }
        if ch == '`' {
            let run = run_length(&chars, index);
            match code {
                Some((open, _)) if open == run => {
                    code = None;
                    last_content = Some(index + run - 1);
                }
                Some(_) => last_content = Some(index + run - 1),
                None => code = Some((run, index + run)),
            }
            index += run;
            continue;
        }
        if code.is_some() {
            last_content = Some(index);
            index += 1;
            continue;
        }

        match ch {
            '*' | '_' | '~' => {
                let run = run_length(&chars, index);
                scan_delimiter(&mut delimiters, ch, run, index, &mut last_content, &at);
                index += run;
            }
            '[' => {
                brackets.push(OpenBracket {
                    index,
                    image: is_image_bracket(&chars, index),
                });
                index += 1;
            }
            ']' => {
                if let Some(open) = brackets.pop() {
                    delimiters.retain(|delimiter| delimiter.opened_at < open.index);
                    if at(index + 1) == Some('(') {
                        let mut scan = index + 2;
                        let mut depth = 0;
                        loop {
                            match at(scan) {
                                Some('(') => depth += 1,
                                Some(')') if depth == 0 => break,
                                Some(')') => depth -= 1,
                                Some(_) => {}
                                None => {
                                    pending_url = Some(PendingDestination {
                                        close_bracket: index,
                                        image: open.image,
                                    });
                                    break;
                                }
                            }
                            scan += 1;
                        }
                        if pending_url.is_some() {
                            break;
                        }
                        last_content = Some(scan);
                        index = scan + 1;
                        continue;
                    }
                }
                last_content = Some(index);
                index += 1;
            }
            ch if ch.is_whitespace() => index += 1,
            _ => {
                last_content = Some(index);
                index += 1;
            }
        }
    }

    let content_end = last_content
        .map(|ix| chars[ix].0 + chars[ix].1.len_utf8())
        .unwrap_or(text.len());

    if let Some(destination) = pending_url {
        // Incomplete images stay literal until their destination is complete.
        if destination.image {
            return None;
        }
        let cut = chars[destination.close_bracket].0;
        let mut closers = String::new();
        close_delimiters(
            &mut closers,
            &delimiters,
            last_content,
            destination.close_bracket,
        );
        let mut mended = String::with_capacity(text.len() + closers.len() + 32);
        mended.push_str(&text[..content_end.min(cut)]);
        mended.push_str(&closers);
        mended.push_str(&text[content_end.min(cut)..cut]);
        mended.push_str("](");
        mended.push_str(PENDING_LINK_URL);
        mended.push(')');
        return Some(MarkdownDisplayRepair {
            markdown: mended,
            synthetic_text_suffix: None,
        });
    }

    let mut closers = String::new();
    if let Some((run, content_at)) = code
        && last_content.is_some_and(|content| content >= content_at)
    {
        closers.extend(std::iter::repeat_n('`', run));
    }
    close_delimiters(&mut closers, &delimiters, last_content, chars.len());
    if let Some(bracket) = brackets.last().copied()
        && last_content.is_some_and(|content| content > bracket.index)
    {
        if bracket.image {
            return None;
        }
        closers.push_str("](");
        closers.push_str(PENDING_LINK_URL);
        closers.push(')');
    }

    let setext_guard = needs_setext_guard(text);
    if closers.is_empty() && !setext_guard {
        return None;
    }

    let mut mended = String::with_capacity(text.len() + closers.len() + 3);
    mended.push_str(&text[..content_end]);
    mended.push_str(&closers);
    mended.push_str(&text[content_end..]);
    if setext_guard {
        mended.push(ZERO_WIDTH_SPACE);
    }
    Some(MarkdownDisplayRepair {
        markdown: mended,
        synthetic_text_suffix: setext_guard.then_some(ZERO_WIDTH_SPACE),
    })
}

/// Return whether `[` is preceded by an unescaped image marker.
fn is_image_bracket(chars: &[(usize, char)], bracket: usize) -> bool {
    let Some(bang) = bracket.checked_sub(1) else {
        return false;
    };
    chars[bang].1 == '!' && !is_escaped(chars, bang)
}

/// Return whether the character at `index` follows an odd backslash run.
fn is_escaped(chars: &[(usize, char)], index: usize) -> bool {
    let mut cursor = index;
    let mut backslashes = 0;
    while cursor > 0 && chars[cursor - 1].1 == '\\' {
        cursor -= 1;
        backslashes += 1;
    }
    backslashes % 2 == 1
}

fn close_delimiters(
    output: &mut String,
    delimiters: &[OpenDelimiter],
    last_content: Option<usize>,
    limit: usize,
) {
    for delimiter in delimiters.iter().rev() {
        if delimiter.opened_at < limit
            && last_content.is_some_and(|content| content >= delimiter.opened_at)
        {
            output.extend(std::iter::repeat_n(delimiter.marker, delimiter.owed));
        }
    }
}

fn run_length(chars: &[(usize, char)], start: usize) -> usize {
    let marker = chars[start].1;
    chars[start..]
        .iter()
        .take_while(|&&(_, ch)| ch == marker)
        .count()
}

fn scan_delimiter(
    delimiters: &mut Vec<OpenDelimiter>,
    marker: char,
    run: usize,
    index: usize,
    last_content: &mut Option<usize>,
    at: &impl Fn(usize) -> Option<char>,
) {
    if marker == '~' && run < 2 {
        *last_content = Some(index + run - 1);
        return;
    }
    let before = index.checked_sub(1).and_then(at);
    let after = at(index + run);
    let can_close = before.is_some_and(|ch| !ch.is_whitespace());
    let can_open = after.is_some_and(|ch| !ch.is_whitespace());
    if can_close
        && let Some(position) = delimiters
            .iter()
            .rposition(|delimiter| delimiter.marker == marker)
        && last_content.is_some_and(|content| content >= delimiters[position].opened_at)
    {
        let owed = delimiters[position].owed;
        if run >= owed {
            delimiters.truncate(position);
        } else {
            delimiters[position].owed = owed - run;
            delimiters.truncate(position + 1);
        }
        *last_content = Some(index + run - 1);
    } else if marker == '_' && before.is_some_and(char::is_alphanumeric) {
        *last_content = Some(index + run - 1);
    } else if can_open {
        delimiters.push(OpenDelimiter {
            marker,
            owed: run,
            opened_at: index + run,
        });
    } else {
        *last_content = Some(index + run - 1);
    }
}

/// Detect a partial Setext underline below ordinary paragraph text.
fn needs_setext_guard(text: &str) -> bool {
    if text.ends_with('\n') {
        return false;
    }
    let mut lines = text.lines().rev();
    let Some(last) = lines.next() else {
        return false;
    };
    let trimmed = last.trim_end();
    if trimmed.is_empty()
        || !(trimmed.chars().all(|ch| ch == '-') || trimmed.chars().all(|ch| ch == '='))
    {
        return false;
    }
    lines.next().is_some_and(|previous| {
        let previous = previous.trim();
        !previous.is_empty() && !previous.starts_with(['-', '=', '#', '>', '`'])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pacer_preserves_graphemes_and_line_boundaries() {
        let mut pacer = StreamingTextPacer::with_config(StreamingTextPacerConfig {
            catch_up_frames: 1,
            min_graphemes_per_frame: 1,
            max_graphemes_per_frame: 256,
            ..Default::default()
        });
        pacer.push_str("a👨‍👩‍👧‍👦\nCafe\u{301}");
        assert_eq!(pacer.take_chunk().as_deref(), Some("a👨‍👩‍👧‍👦\n"));
        assert_eq!(pacer.drain(), "Cafe\u{301}");
    }

    #[test]
    fn pacer_adapts_to_backlog_and_preserves_order() {
        let mut pacer = StreamingTextPacer::new();
        let source = "x".repeat(500);
        pacer.push_str(&source);
        let first = pacer.take_chunk().unwrap();
        assert!((12..=256).contains(&first.chars().count()));
        let rebuilt = first + &pacer.drain();
        assert_eq!(rebuilt, source);
    }

    #[test]
    fn pacer_preserves_final_unicode_sequence_across_provider_deltas() {
        let mut pacer = StreamingTextPacer::with_config(StreamingTextPacerConfig {
            catch_up_frames: 1,
            min_graphemes_per_frame: 1,
            max_graphemes_per_frame: 1,
            ..Default::default()
        });
        pacer.push_str("e");
        let first = pacer.take_chunk().unwrap();
        pacer.push_str("\u{301}");

        assert_eq!(first + &pacer.drain(), "e\u{301}");
    }

    #[test]
    fn mender_closes_common_inline_tails() {
        assert_eq!(
            close_hanging_markdown("**bold").unwrap().markdown,
            "**bold**"
        );
        assert_eq!(close_hanging_markdown("`code").unwrap().markdown, "`code`");
        assert_eq!(
            close_hanging_markdown("[docs](https://exa")
                .unwrap()
                .markdown,
            format!("[docs]({PENDING_LINK_URL})")
        );
    }

    #[test]
    fn mender_keeps_incomplete_images_literal() {
        assert_eq!(close_hanging_markdown("![alt"), None);
        assert_eq!(close_hanging_markdown("![alt](https://exa"), None);
        assert_eq!(
            close_hanging_markdown("\\![literal").unwrap().markdown,
            format!("\\![literal]({PENDING_LINK_URL})")
        );
    }

    #[test]
    fn mender_ignores_escaped_markers() {
        assert_eq!(close_hanging_markdown("\\*literal"), None);
        assert_eq!(close_hanging_markdown("word_within"), None);
        assert_eq!(close_hanging_markdown("---"), None);
        let setext = close_hanging_markdown("heading\n---").unwrap();
        assert_eq!(setext.synthetic_text_suffix, Some(ZERO_WIDTH_SPACE));
        assert!(setext.markdown.ends_with(ZERO_WIDTH_SPACE));
    }
}
