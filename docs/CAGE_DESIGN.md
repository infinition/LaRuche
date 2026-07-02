# LaRuche - Cage : sandbox dure cross-platform pour l'exécution shell

> Design de la « sandbox shell dure » (dernier item sécurité ouvert de la roadmap).
> Contrainte posée par Fabien : LaRuche vise TOUS les OS (Windows, macOS, Linux).
> Le gate d'approbation humain reste le contrôle d'INTENTION ; la cage fait
> respecter les limites par l'OS lui-même : même une commande approuvée par
> erreur ne peut pas dépasser sa cage.

## Principes

1. **Une abstraction, trois implémentations.** Un trait unique appliqué au spawn
   dans les trois outils concernés (`shell_exec`, `execute_code`, `run_script`),
   avec une implémentation par OS derrière `#[cfg(target_os)]`.
2. **Dégradation honnête.** Chaque capacité de cage est déclarée : si un OS ne
   sait pas confiner le filesystem, on le dit (log + doctor), on ne fait pas
   semblant. Échec de pose de cage = warn visible + exécution non cagée par
   défaut (fail-open), avec `LARUCHE_CAGE=strict` pour refuser d'exécuter sans
   cage (fail-closed).
3. **La blocklist de motifs reste un ralentisseur assumé**, la cage ne la
   remplace pas : elles couvrent des choses différentes (intention vs dégâts).

## Architecture

- Nouveau crate `laruche-cage` (workspace member), zéro dépendance sur essaim :
  ```rust
  pub struct LimitesCage {
      pub memoire_max_mo: u64,      // défaut 1024
      pub processus_max: u32,       // défaut 32 (anti fork-bomb)
      pub cpu_secs_max: u64,        // aligné sur le timeout de l'outil
      pub dossier_travail: PathBuf, // jail de départ
  }
  pub struct RapportCage {
      pub memoire: bool,   // la limite mémoire est réellement posée
      pub processus: bool,
      pub cpu: bool,
      pub filesystem: bool, // confinement FS réel (Linux Landlock seulement, phase 2)
  }
  /// Pose la cage sur une Command AVANT spawn ; retourne ce qui est
  /// effectivement garanti sur cet OS.
  pub fn encager(cmd: &mut tokio::process::Command, limites: &LimitesCage)
      -> anyhow::Result<RapportCage>;
  /// À la fin : tue TOUT l'arbre de processus (pas seulement l'enfant direct).
  pub fn tuer_arbre(enfant: &mut tokio::process::Child);
  ```
- Câblage dans les outils : `encager()` avant spawn, `tuer_arbre()` au timeout
  (aujourd'hui seul l'enfant direct meurt, ses petits-enfants survivent : c'est
  un trou sur les trois OS).
- Le `RapportCage` du dernier run est exposé au doctor (`/api/doctor`) : ligne
  « Cage shell » honnête par OS.

## Par OS

### Phase 1 - emballement contenu partout (mémoire, processus, CPU, arbre tué)

| OS | Mécanisme | Crate |
|---|---|---|
| Windows | **Job Object** : `JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `ACTIVE_PROCESS`, `JOB_OBJECT_LIMIT_JOB_TIME`, `KILL_ON_JOB_CLOSE` (l'arbre entier meurt avec la job, y compris au crash du node) | `windows` |
| Linux | `pre_exec` : `setrlimit` (`RLIMIT_AS`, `RLIMIT_NPROC`, `RLIMIT_CPU`) + `setsid` (groupe de processus dédié -> kill(-pgid)) | `libc`/`rustix` |
| macOS | idem Linux : `setrlimit` + `setsid` + kill du groupe (`RLIMIT_NPROC` est respecté ; `RLIMIT_AS` partiellement -> compléter par surveillance RSS du groupe au tick du timeout) | `libc` |

Notes phase 1 :
- `pre_exec` est `unsafe` (post-fork) : uniquement des appels async-signal-safe.
- Windows : créer le process SUSPENDU, l'assigner à la job, puis resume, sinon
  il peut forker avant l'assignation.
- macOS sans limite mémoire dure fiable : le rapport dit `memoire: false` si le
  fallback RSS est utilisé, on ne ment pas.

### Phase 2 - confinement filesystem (là où l'OS le permet sans privilèges)

| OS | Mécanisme | Réalisme |
|---|---|---|
| Linux | **Landlock** (kernel >= 5.13, non privilégié) : lecture seule hors du dossier de travail, écriture uniquement dedans | très bon, crate `landlock` |
| macOS | `sandbox-exec -p` avec profil généré (déprécié par Apple mais fonctionnel) ; sinon rester phase 1 | moyen, à valider par version d'OS |
| Windows | token restreint ou AppContainer | coûteux ; différer, la job phase 1 couvre déjà l'essentiel des dégâts involontaires |

### Explicitement hors périmètre
- Réseau : pas de filtrage par la cage (le proxy/pare-feu de l'OS reste le bon
  outil) ; noté dans le doctor.
- Conteneurs/WSL : refusés, cassent le local-first double-clic.

## Configuration

- `EssaimConfig` : `cage_memoire_mo` (1024), `cage_processus_max` (32),
  exposés dans Settings > Avancé.
- `LARUCHE_CAGE=off|on|strict` (défaut `on`) : off = comportement actuel,
  strict = fail-closed si la cage ne peut pas être posée.
- La délégation (scouts) hérite de la cage du parent.

## Ordre d'implémentation (session dédiée)

1. Crate `laruche-cage` + `tuer_arbre` cross-platform (le trou le plus simple
   et le plus réel : l'arbre survivant au timeout).
2. Windows Job Object (l'OS de dev : testable immédiatement).
3. Linux/macOS rlimits + setsid (mêmes chemins de code, `cfg` séparés).
4. Câblage shell_exec / execute_code / run_script + doctor + Settings.
5. Phase 2 Landlock (Linux) en option de la même session si le temps le permet.

## Tests

- Par OS : fork-bomb apprivoisée (spawn N enfants -> compte plafonné), alloc
  au-delà du plafond (tuée), boucle infinie (CPU/timeout, ARBRE mort vérifié),
  écriture hors jail (phase 2 Linux : refusée).
- CI : les tests de cage sont `#[cfg]`-gated par OS et marqués `#[ignore]` en
  environnement non privilégié si nécessaire ; le harnais local de Fabien
  (Windows) couvre la job object en continu.
