//! Non-blocking orchestrator for the review after an agent turn.
//!
//! The integration point provides only two specialized futures: a memory pass and
//! a skill pass. The session, prompt cache, and Abeille registry stay out of
//! scope, which excludes general tools (shell, files, network, etc.).

use anyhow::Result;
use std::future::Future;

/// The only action categories accessible to the background reviewer.
pub const BACKGROUND_REVIEW_ACTIONS: &[&str] = &["memory_write", "skill_propose"];

/// Runs both decisions of the mini-reviewer without blocking the user turn.
///
/// Errors are isolated: a memory curation failure must never prevent the
/// proposal of a skill, and vice versa. The calling futures only have access to the
/// memory/skill stores; they receive neither session nor tool registry.
pub(crate) async fn run_background_review<MemoryReview, SkillReview>(
    memory_review: MemoryReview,
    skill_review: SkillReview,
) where
    MemoryReview: Future<Output = Result<()>>,
    SkillReview: Future<Output = Result<()>>,
{
    if let Err(error) = memory_review.await {
        tracing::warn!(error = %error, "background memory review skipped");
    }
    if let Err(error) = skill_review.await {
        tracing::warn!(error = %error, "background skill review skipped");
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
            async { Err(anyhow::anyhow!("memory unavailable")) },
            async move {
                skill_runs_for_future.fetch_add(1, Ordering::Relaxed);
                Ok(())
            },
        )
        .await;

        assert_eq!(skill_runs.load(Ordering::Relaxed), 1);
    }
}
