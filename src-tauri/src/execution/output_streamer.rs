use tokio::sync::mpsc;

/// The type of stream an output line belongs to.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamType {
    Stdout,
    Stderr,
    /// A progress percentage (0–100), parsed from cargo / npm / pip output.
    Progress(u8),
}

/// A single line of output produced by a running process.
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub line: String,
    pub stream: StreamType,
    pub timestamp: std::time::Instant,
}

/// Streams command output line-by-line (and optionally raw bytes) over a
/// Tokio MPSC channel so callers can react to each line in real-time.
pub struct OutputStreamer {
    line_tx: mpsc::UnboundedSender<OutputLine>,
    /// Raw bytes accumulated between line breaks.
    buffer: Vec<u8>,
}

impl OutputStreamer {
    /// Create a new streamer pair.  The caller holds onto `rx` to receive
    /// lines as they are produced.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<OutputLine>) {
        let (line_tx, rx) = mpsc::unbounded_channel();
        let streamer = Self {
            line_tx,
            buffer: Vec::new(),
        };
        (streamer, rx)
    }

    // ── line-by-line streaming ──────────────────────────────────────────

    /// Feed a chunk of stdout bytes.  Lines are emitted immediately.
    pub fn feed_stdout(&mut self, chunk: &[u8]) {
        self.feed(chunk, StreamType::Stdout);
    }

    /// Feed a chunk of stderr bytes.  Lines are emitted immediately.
    pub fn feed_stderr(&mut self, chunk: &[u8]) {
        self.feed(chunk, StreamType::Stderr);
    }

    /// Feed raw bytes and emit complete lines through the channel.
    fn feed(&mut self, chunk: &[u8], stream: StreamType) {
        self.buffer.extend_from_slice(chunk);

        // Extract complete lines (split on \n, but also handle \r\n for Windows)
        let mut i = 0;
        while i < self.buffer.len() {
            if self.buffer[i] == b'\n' {
                let line_bytes = &self.buffer[..i];
                let line_str = String::from_utf8_lossy(line_bytes)
                    .trim_end_matches('\r')
                    .to_string();
                let _ = self.line_tx.send(OutputLine {
                    line: line_str,
                    stream: stream.clone(),
                    timestamp: std::time::Instant::now(),
                });
                i += 1;
                // skip \r if present
                if i < self.buffer.len() && self.buffer[i] == b'\r' {
                    i += 1;
                }
                self.buffer.drain(..i);
                i = 0;
            } else {
                i += 1;
            }
        }
    }

    /// Flush any remaining buffered bytes as a final line (even if no trailing \n).
    pub fn flush(&mut self) {
        if !self.buffer.is_empty() {
            let line_str = String::from_utf8_lossy(&self.buffer).to_string();
            let _ = self.line_tx.send(OutputLine {
                line: line_str,
                stream: StreamType::Stdout,
                timestamp: std::time::Instant::now(),
            });
            self.buffer.clear();
        }
    }

    // ── progress reporting ──────────────────────────────────────────────

    /// Convenience: send a progress update.
    pub fn report_progress(&self, percent: u8) {
        let _ = self.line_tx.send(OutputLine {
            line: format!("{}%", percent),
            stream: StreamType::Progress(percent.min(100)),
            timestamp: std::time::Instant::now(),
        });
    }

    /// Try to parse a progress percentage from common build tool output
    /// (e.g. cargo: `Compiling foo v0.1.0`, npm: `[1/5] Installing…`).
    /// Returns `Some(percent)` if a recognizable pattern is found.
    pub fn parse_progress(line: &str) -> Option<u8> {
        // npm/yarn style: `[1/5]`
        let re_npm = regex::Regex::new(r"\[(\d+)/(\d+)\]").ok()?;
        if let Some(caps) = re_npm.captures(line) {
            let cur: f64 = caps.get(1)?.as_str().parse().ok()?;
            let total: f64 = caps.get(2)?.as_str().parse().ok()?;
            if total > 0.0 {
                return Some(((cur / total) * 100.0) as u8);
            }
        }

        // cargo style: `Compiling (1/100)` or similar
        let re_cargo = regex::Regex::new(r"\((\d+)/(\d+)\)").ok()?;
        if let Some(caps) = re_cargo.captures(line) {
            let cur: f64 = caps.get(1)?.as_str().parse().ok()?;
            let total: f64 = caps.get(2)?.as_str().parse().ok()?;
            if total > 0.0 {
                return Some(((cur / total) * 100.0) as u8);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feed_stdout_single_line() {
        let (mut streamer, mut rx) = OutputStreamer::new();
        streamer.feed_stdout(b"hello world\n");
        let line = rx.try_recv().expect("should have received a line");
        assert_eq!(line.line, "hello world");
        assert_eq!(line.stream, StreamType::Stdout);
    }

    #[test]
    fn test_feed_stderr() {
        let (mut streamer, mut rx) = OutputStreamer::new();
        streamer.feed_stderr(b"error: something broke\n");
        let line = rx.try_recv().expect("should have received a line");
        assert_eq!(line.line, "error: something broke");
        assert_eq!(line.stream, StreamType::Stderr);
    }

    #[test]
    fn test_multiple_lines_in_chunk() {
        let (mut streamer, mut rx) = OutputStreamer::new();
        streamer.feed_stdout(b"line1\nline2\nline3\n");

        assert_eq!(rx.try_recv().unwrap().line, "line1");
        assert_eq!(rx.try_recv().unwrap().line, "line2");
        assert_eq!(rx.try_recv().unwrap().line, "line3");
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_line_split_across_chunks() {
        let (mut streamer, mut rx) = OutputStreamer::new();
        streamer.feed_stdout(b"hel");
        streamer.feed_stdout(b"lo\n");
        let line = rx.try_recv().expect("should have received a line");
        assert_eq!(line.line, "hello");
    }

    #[test]
    fn test_flush_remaining() {
        let (mut streamer, mut rx) = OutputStreamer::new();
        streamer.feed_stdout(b"no newline");
        assert!(rx.try_recv().is_err());

        streamer.flush();
        let line = rx.try_recv().expect("flush should emit remaining");
        assert_eq!(line.line, "no newline");
    }

    #[test]
    fn test_progress_report() {
        let (streamer, mut rx) = OutputStreamer::new();
        streamer.report_progress(42);
        let line = rx.try_recv().expect("should have received progress");
        assert_eq!(line.stream, StreamType::Progress(42));
        assert_eq!(line.line, "42%");
    }

    #[test]
    fn test_parse_progress_npm_style() {
        assert_eq!(
            OutputStreamer::parse_progress("[1/4] Installing..."),
            Some(25)
        );
        assert_eq!(
            OutputStreamer::parse_progress("[2/4] Installing..."),
            Some(50)
        );
        assert_eq!(OutputStreamer::parse_progress("[4/4] Done!"), Some(100));
    }

    #[test]
    fn test_parse_progress_cargo_style() {
        assert_eq!(
            OutputStreamer::parse_progress("Compiling foo (1/10)"),
            Some(10)
        );
        assert_eq!(
            OutputStreamer::parse_progress("Compiling foo (10/10)"),
            Some(100)
        );
    }

    #[test]
    fn test_parse_progress_no_match() {
        assert_eq!(OutputStreamer::parse_progress("hello world"), None);
        assert_eq!(OutputStreamer::parse_progress(""), None);
    }

    #[test]
    fn test_windows_crlf_handling() {
        let (mut streamer, mut rx) = OutputStreamer::new();
        streamer.feed_stdout(b"line1\r\nline2\r\n");
        assert_eq!(rx.try_recv().unwrap().line, "line1");
        assert_eq!(rx.try_recv().unwrap().line, "line2");
    }
}
