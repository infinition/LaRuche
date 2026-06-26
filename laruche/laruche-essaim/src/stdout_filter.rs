//! Filtre intelligent pour les sorties longues de scripts.
//!
//! Évite de noyer le LLM avec des milliers de lignes de log.
//! Garde les lignes importantes (erreurs, warnings, progrès)
//! et produit un résumé glissant.

/// Filtre intelligent pour stdout long.
/// Conserve les lignes signal importantes et résume périodiquement.
pub struct StdoutFilter {
    buffer: Vec<String>,
    line_count: usize,
    /// Nombre de lignes envoyées au LLM (pour garder une trace)
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

    /// Traite une ligne de stdout. Retourne `Some(line)` si la ligne doit
    /// être transmise au LLM, `None` si elle peut être ignorée.
    pub fn process_line(&mut self, line: &str) -> Option<String> {
        self.line_count += 1;

        // Lignes signal importantes — toujours transmises
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

        // Résumé glissant tous les 100 tours
        if self.line_count % 100 == 0 {
            let summary = format!(
                "[Ligne {}] ... (100 lignes silencieuses) Dernier log: {}",
                self.line_count,
                line.chars().take(120).collect::<String>()
            );
            self.emitted_lines += 1;
            self.buffer.clear();
            return Some(summary);
        }

        // Garder dans le buffer pour le résumé final
        self.buffer.push(line.to_string());
        if self.buffer.len() > 10 {
            self.buffer.remove(0);
        }

        None
    }

    /// Résumé final quand le script se termine.
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
            out.push_str(&format!(" Dernière ligne: {}", tail));
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

        // Vérifie qu'on a un résumé autour de la ligne 200
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
