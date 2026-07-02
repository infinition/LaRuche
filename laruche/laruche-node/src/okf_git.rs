//! Memory time-travel: periodic OKF export committed into a DEDICATED git
//! repository (`memoire-okf/`, its own history, independent from the code
//! repo). Every snapshot that actually changed something becomes a commit, so
//! the whole cognitive map gains diff and rollback for free (`git log`,
//! `git diff`, `git checkout <sha> -- <file>` then re-import). First slice of
//! the "OKF + git" roadmap item; mesh fact federation builds on it later.

use crate::*;
use std::path::Path;
use std::sync::Arc;

/// One snapshot: export the OKF bundle into `dir`, and commit it when anything
/// changed. Returns `Ok(Some(resume))` on a new commit, `Ok(None)` when the
/// memory was unchanged (no commit).
pub(crate) async fn snapshot(
    memoire: &Arc<dyn laruche_memoire::MemoireCognitive>,
    dir: &Path,
) -> anyhow::Result<Option<String>> {
    let fichiers = memoire.export_okf(dir, None).await?;
    // The export stamps every file with the EXPORT time: neutralize that
    // volatile line so an unchanged memory produces a byte-identical tree
    // (otherwise every snapshot would look dirty and commit for nothing).
    neutraliser_timestamps(dir)?;

    // Dedicated repo: init once, with a local identity so commit never depends
    // on the user's global git config.
    if !dir.join(".git").exists() {
        git(dir, &["init", "-q"]).await?;
        git(dir, &["config", "user.name", "LaRuche"]).await?;
        git(dir, &["config", "user.email", "laruche@local"]).await?;
    }

    let statut = git(dir, &["status", "--porcelain"]).await?;
    if statut.trim().is_empty() {
        return Ok(None);
    }
    let changements = statut.lines().count();
    git(dir, &["add", "-A"]).await?;
    let message = format!("memoire: snapshot ({fichiers} fichiers OKF, {changements} changement(s))");
    git(dir, &["commit", "-q", "-m", &message]).await?;
    Ok(Some(message))
}

/// Rewrites `timestamp: <export time>` frontmatter lines to a stable marker in
/// every exported .md file (recursive). Git history dates the snapshots; the
/// per-file export time is noise here.
fn neutraliser_timestamps(dir: &Path) -> anyhow::Result<()> {
    for entree in std::fs::read_dir(dir)? {
        let entree = entree?;
        let chemin = entree.path();
        if chemin.file_name().map(|n| n == ".git").unwrap_or(false) {
            continue;
        }
        if chemin.is_dir() {
            neutraliser_timestamps(&chemin)?;
        } else if chemin.extension().map(|e| e == "md").unwrap_or(false) {
            let contenu = std::fs::read_to_string(&chemin)?;
            let propre: String = contenu
                .lines()
                .map(|l| {
                    if l.starts_with("timestamp: ") {
                        "timestamp: (snapshot)"
                    } else {
                        l
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            if propre != contenu {
                std::fs::write(&chemin, propre)?;
            }
        }
    }
    Ok(())
}

async fn git(dir: &Path, args: &[&str]) -> anyhow::Result<String> {
    let sortie = tokio::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("git unavailable: {e}"))?;
    if !sortie.status.success() {
        anyhow::bail!(
            "git {:?} failed: {}",
            args.first().unwrap_or(&"?"),
            String::from_utf8_lossy(&sortie.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&sortie.stdout).to_string())
}

/// Background job: a snapshot every `LARUCHE_OKF_GIT_SECS` (default 30 min,
/// `0` disables). First pass deferred so startup stays light. A missing git
/// binary is logged once and the job stops (graceful degradation).
pub(crate) fn spawn_okf_git(state: &Arc<AppState>) {
    let secs: u64 = std::env::var("LARUCHE_OKF_GIT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);
    if secs == 0 {
        return;
    }
    let okf_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(secs));
        let dir = std::path::PathBuf::from("memoire-okf");
        loop {
            interval.tick().await;
            match snapshot(&okf_state.memoire, &dir).await {
                Ok(Some(resume)) => {
                    info!(%resume, "OKF git snapshot committed");
                    laruche_essaim::feed_journal::record(
                        "LaRuche",
                        "memory",
                        "committed a time-travel snapshot",
                        resume,
                        chrono::Utc::now(),
                    );
                }
                Ok(None) => {}
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("git unavailable") {
                        warn!("OKF git snapshots disabled: {msg}");
                        return; // no git binary: stop the job for this run
                    }
                    warn!(error = %msg, "OKF git snapshot failed");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use laruche_memoire::{MemoireCognitive, MemoryItem, SqliteBackend};

    #[tokio::test]
    async fn snapshot_commite_puis_reste_silencieux_sans_changement() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("laruche_okfgit_{stamp}"));
        let db = base.join("memoire.db");
        std::fs::create_dir_all(&base).unwrap();
        let mem = SqliteBackend::open(&db).unwrap();
        mem.write(MemoryItem::new("projets.okf", "fait durable pour le time-travel"))
            .await
            .unwrap();
        let mem: Arc<dyn MemoireCognitive> = Arc::new(mem);
        let repo = base.join("okf");

        // First snapshot: creates the repo and commits.
        let premier = snapshot(&mem, &repo).await.unwrap();
        assert!(premier.is_some(), "first snapshot must commit");
        assert!(repo.join(".git").exists());

        // No memory change: no commit.
        assert!(snapshot(&mem, &repo).await.unwrap().is_none());

        // A new fact: a second commit.
        mem.write(MemoryItem::new("projets.okf", "second fait, second commit"))
            .await
            .unwrap();
        assert!(snapshot(&mem, &repo).await.unwrap().is_some());
        let log = git(&repo, &["log", "--oneline"]).await.unwrap();
        assert_eq!(log.lines().count(), 2, "{log}");

        let _ = std::fs::remove_dir_all(&base);
    }
}
