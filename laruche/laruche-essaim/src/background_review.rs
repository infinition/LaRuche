//! Orchestrateur non bloquant de la revue après un tour d'agent.
//!
//! Le point d'intégration ne fournit que deux futures spécialisées : une passe mémoire et
//! une passe skill. La session, le cache de prompt et le registre d'Abeilles restent hors de
//! portée, ce qui exclut les outils généraux (shell, fichiers, réseau…).

use anyhow::Result;
use std::future::Future;

/// Les seules catégories d'action accessibles au reviewer de fond.
pub const BACKGROUND_REVIEW_ACTIONS: &[&str] = &["memory_write", "skill_propose"];

/// Exécute les deux décisions du mini-reviewer sans bloquer le tour utilisateur.
///
/// Les erreurs sont isolées : un échec de curation mémoire ne doit jamais empêcher la
/// proposition d'un skill, et inversement. Les futures appelantes n'ont accès qu'aux stores
/// mémoire/skill ; elles ne reçoivent ni session ni registre d'outils.
pub(crate) async fn run_background_review<MemoryReview, SkillReview>(
    memory_review: MemoryReview,
    skill_review: SkillReview,
) where
    MemoryReview: Future<Output = Result<()>>,
    SkillReview: Future<Output = Result<()>>,
{
    if let Err(error) = memory_review.await {
        tracing::warn!(error = %error, "background review mémoire ignorée");
    }
    if let Err(error) = skill_review.await {
        tracing::warn!(error = %error, "background review skill ignorée");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn reviewer_is_restricted_to_memory_and_skill_actions() {
        assert_eq!(BACKGROUND_REVIEW_ACTIONS, ["memory_write", "skill_propose"]);
    }

    #[tokio::test]
    async fn a_failed_memory_pass_does_not_skip_skill_review() {
        let skill_runs = Arc::new(AtomicUsize::new(0));
        let skill_runs_for_future = skill_runs.clone();
        run_background_review(
            async { Err(anyhow::anyhow!("mémoire indisponible")) },
            async move {
                skill_runs_for_future.fetch_add(1, Ordering::Relaxed);
                Ok(())
            },
        )
        .await;

        assert_eq!(skill_runs.load(Ordering::Relaxed), 1);
    }
}
