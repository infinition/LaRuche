//! Smart filter for long script outputs.
//!
//! Avoids drowning the LLM in thousands of log lines.
//! Keeps important lines (errors, warnings, progress)
//! and produces a sliding summary.

/// Smart filter for long stdout.
/// Keeps important signal lines and summarizes periodically.
pub struct StdoutFilter {
    buffer: Vec<String>,
    line_count: usize,
    /// Number of lines sent to the LLM (kept for tracking)
    emitted_lines: usize,
}

impl StdoutFilter {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            line_count: 0,
            emitted_lines: 0,
        }
    }

    /// Processes a stdout line. Returns `Some(line)` if the line should
    /// be forwarded to the LLM, `None` if it can be ignored.
    pub fn process_line(&mut self, line: &str) -> Option<String> {
        self.line_count += 1;

        // Important signal lines, always forwarded
        let is_signal = line.contains("Error")
            || line.contains("error")
            || line.contains("Warning")
            || line.contains("FAILED")
            || line.contains("✓")
            || line.contains("✗")
            || line.contains("100%")
            || line.contains("Complete")
            || line.contains("Traceback")
            || line.contains("assertion")
            || line.contains("panic")
            || line.contains("Killed")
            || line.contains("OOM")
            || line.contains("fatal");

        if is_signal {
            self.emitted_lines += 1;
            return Some(format!("#{} {}", self.line_count, line));
        }

        // Sliding summary every 100 rounds
        if self.line_count % 100 == 0 {
            let summary = format!(
                "[Line {}] ... (100 silent lines) Last log: {}",
                self.line_count,
                line.chars().take(120).collect::<String>()
            );
            self.emitted_lines += 1;
            self.buffer.clear();
            return Some(summary);
        }

        // Keep in the buffer for the final summary
        self.buffer.push(line.to_string());
        if self.buffer.len() > 10 {
            self.buffer.remove(0);
        }

        None
    }

    /// Final summary when the script ends.
    pub fn final_summary(&self, exit_code: i32) -> String {
        let total = self.line_count;
        let emitted = self.emitted_lines;
        let tail: String = self
            .buffer
            .last()
            .map(|l| l.chars().take(200).collect())
            .unwrap_or_default();

        let mut out = format!(
            "[Script terminé] Exit code: {}. {} lignes totales, {} lignes transmises.",
            exit_code, total, emitted
        );
        if !tail.is_empty() {
            out.push_str(&format!(" Last line: {}", tail));
        }
        out
    }
}

impl Default for StdoutFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtre_lignes_importantes() {
        let mut f = StdoutFilter::new();

        assert!(f.process_line("Error: something failed").is_some());
        assert!(f.process_line("100% completed").is_some());
        assert!(f.process_line("normal log line").is_none());
    }

    #[test]
    fn resumé_glissant_tous_les_100() {
        let mut f = StdoutFilter::new();

        let mut last_output = None;
        for i in 1..210 {
            let l = f.process_line(&format!("step {i}"));
            if l.is_some() {
                last_output = l;
            }
        }

        // Check that we have a summary around line 200
        let summary = last_output.unwrap_or_default();
        assert!(summary.contains("Ligne 200") || summary.contains("200"));
    }

    #[test]
    fn resumé_final_inclut_exit_code() {
        let mut f = StdoutFilter::new();
        for i in 1..=5 {
            let _ = f.process_line(&format!("line {i}"));
        }
        let s = f.final_summary(0);
        assert!(s.contains("Exit code: 0"));
        assert!(s.contains("5 lignes totales"));
    }
}
