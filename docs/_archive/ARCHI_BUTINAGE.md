# Architecture « Butinage » — le nouveau cerveau de LaRuche

> Réécriture du moteur ReAct (`laruche-essaim/src/brain.rs`, 4458 l.) en un crate
> dédié `laruche-butinage`, à nomenclature **native ruche** (aucun nom emprunté à
> third-party / third-party / Claude Code). Cohabite avec l'ancien derrière un flag, puis le
> remplace. Objectifs : longévité, sous-agents, recherche longue, zéro arrêt inutile,
> tous les cas d'erreur/reprise couverts.

## 0. Métaphore = DSL du code

L'agent est une **abeille butineuse** ; une mission est un **butinage** (quête itérative).

| Concept agentique générique | Notre nom | Rôle |
|---|---|---|
| Moteur / boucle ReAct | **`butinage`** (crate), `butiner()` | la quête itérative |
| Une itération / tour | **`Passe`** | un aller-retour vers une fleur |
| Issue d'une passe (stop reason interne) | **`Issue`** (enum) | Poursuit / Pose / Tronquée / Bloquée / Délègue / Clarifie |
| Fin terminale d'un butinage | **`FinDeVol`** | Accomplie / Plafond / Erreur / Interrompue |
| État persistable (reprise crash) | **`Carnet`** | carnet de bord sérialisable |
| Plan / todo = source de vérité | **`Itineraire`** | étapes ordonnées, statut par étape |
| Contrôleur anti-boucle **pur** | **`Vigie`** | observe, renvoie un `Signal` (sans effet de bord) |
| Décision de la Vigie | **`Signal`** | Laisser / Avertir / Bloquer / Poser |
| Politique de continuation | **`boussole::cap()`** | UNE fn testée : continuer/poser/relancer |
| Budget contexte (tokens réels) | **`Jauge`** | usage provider réel, pas `len/4` |
| Hook entre passes (compaction/consolidation) | **`Escale`** | halte pour « faire le miel » |
| Exécution outils + partition ∥/séq | **`Recolte`**, `recolter()` | récolte du pollen |
| Fournisseur de contexte (mémoire) injecté | **`trait Source`** (nectar) | just-in-time retrieval |
| Sous-agent | **`Eclaireuse`** | abeille éclaireuse (scout) |
| Orchestration de sous-agents | **`essaimage`** | dépêcher des éclaireuses |
| Message inter-agents (mesh) | **`Danse`** | danse frétillante (waggle dance) |
| Classification d'erreurs + retry | **`Meteo`** | conditions de vol → politique |
| Bascule de modèle/clé | **`deroutement`** | déroutement vers une autre fleur |
| Résultat final d'un butinage | **`Bilan`** | texte + métriques + faits consolidés |

Règle de lisibilité : **types/modules** portent le nom-ruche ; **méthodes** restent explicites
(`cap()`, `recolter()`, `consommer()`), jamais cryptiques.

## 1. Arborescence du crate

```
laruche-butinage/
├── Cargo.toml
└── src/
    ├── lib.rs          // expose butiner() + adaptateurs de compat (contrat node)
    ├── cycle.rs        // LA boucle, ~180 l. Pilotée par Issue. Zéro heuristique métier.
    ├── carnet.rs       // Carnet : état persistable (itinéraire, compteurs, jauge, météo)
    ├── issue.rs        // enum Issue, enum FinDeVol, struct Bilan
    ├── itineraire.rs   // plan/todo structuré = source de vérité de la terminaison
    ├── cap/
    │   ├── mod.rs
    │   ├── vigie.rs    // contrôleur PUR anti-boucle (Signal)
    │   ├── boussole.rs // fn cap(...) -> Decision  (la SEULE politique de continuation)
    │   └── jauge.rs    // budget tokens réels (usage provider)
    ├── escale.rs       // entre passes : compaction | consolidation (système unifié)
    ├── recolte.rs      // partition read-only ∥ (Semaphore) / mutating séquentiel
    ├── nectar.rs       // trait Source (mémoire injectée, optionnelle)
    ├── eclaireuse.rs   // sous-agents : rôles, budget séparé, stream remonté
    ├── meteo.rs        // ErrorClass + retry/backoff/déroutement
    └── prompt.rs       // assemblage 3 tiers (stable/contexte/volatile) cacheable
```

Réutilisé **tel quel** (dépendances, non réécrit) : `laruche-permissions`, `laruche-memoire`,
`credential_pool`, `providers`, `codex_auth`, `threat_patterns`, `abeille::{Abeille,Registry}`.

## 2. Boucle `cycle.rs` — bête, pilotée par `Issue`

Le principe d'third-party : **boucle minimale, complexité déléguée à des hooks**. Pseudocode :

```rust
pub async fn butiner(mission: &str, carnet: &mut Carnet, ruche: &Ruche) -> Result<Bilan> {
    carnet.itineraire.amorcer(mission);
    loop {                                   // une passe = une itération
        if carnet.passe >= ruche.plafond_passes { return Ok(carnet.bilan(FinDeVol::Plafond)); }
        injecter_steering(carnet, ruche);                       // messages live utilisateur
        escale::peut_etre(carnet, ruche).await?;                // compaction/consolidation si Jauge le dit
        let msgs = prompt::assembler(carnet, ruche);            // 3 tiers, préfixe stable cacheable
        let reponse = match meteo::stream_robuste(&msgs, carnet, ruche).await {
            Ok(r) => r,                                         // retry/rotation clé/déroutement gérés dans meteo
            Err(fatal) => return Ok(carnet.bilan(FinDeVol::Erreur(fatal))),
        };
        let issue = analyser(reponse, carnet);                 // parse tool_calls + plan + stop_reason
        match boussole::cap(carnet, &issue) {                  // ← LA politique, ailleurs et testée
            Decision::Poser(fin)      => return Ok(carnet.bilan(fin)),
            Decision::Relancer(nudge) => { carnet.injecter(nudge); continue; }   // borné par Vigie
            Decision::Recolter(appels)=> {
                let obs = recolte::recolter(appels, carnet, ruche).await;        // ∥ read-only / séq mutating
                carnet.observer(obs);                                            // + Vigie.après_appel()
            }
            Decision::Deleguer(ordre) => {
                let rapport = eclaireuse::depecher(ordre, ruche).await;          // sous-agent isolé, streamé
                carnet.observer_rapport(rapport);
            }
            Decision::Clarifier(q)    => return Ok(carnet.bilan(FinDeVol::Clarification(q))),
        }
        carnet.passe += 1;
        carnet.sauver().await;                                  // checkpoint → reprise après crash
    }
}
```

Tout `if reponse_signale_fin()` / `reponse_negative_recherche()` / Dungeon Siege **disparaît** :
remplacé par `boussole::cap()` (testable) + outil explicite `mission_accomplie` + `Itineraire`.

## 3. `Issue` & terminaison — fin de la terminaison par string-matching

```rust
enum Issue {
    ToolCalls(Vec<Appel>),     // outils à exécuter
    MissionAccomplie(Bilan),   // outil explicite mission_accomplie(résumé, confiance)
    Clarification(String),     // outil clarify → rend la main
    Delegation(OrdreEclaireuse),
    TexteSeul { fin_native: Option<StopReason>, plan_inacheve: bool, malforme: bool, tronquee: bool },
}
```

`boussole::cap` décide selon des **faits**, pas des chaînes :
- `MissionAccomplie` **ou** (`TexteSeul.fin_native == EndTurn` **et** `itineraire.tout_termine()`) → **Poser(Accomplie)**.
- `itineraire` a des étapes ouvertes **et** budget auto-continuation restant → **Relancer** (« étape suivante »).
- `tronquee` (`stop_reason == length`) → **Relancer**(reprise exacte), borné à N.
- `malforme` (ressemble à un tool_call cassé) → **Relancer**(reformate), borné.
- `Vigie` renvoie `Poser` (boucle stérile détectée) → **Poser(Erreur)** propre.

Mode **recherche longue** : pas une heuristique de prompt, mais un *attribut de mission*
(`ruche.mode == Exploration`) qui (a) relève le seuil d'auto-continuation, (b) demande à la
Vigie de traiter « rien trouvé » comme *checkpoint d'étape* tant que `itineraire` a des axes non
explorés, (c) autorise l'essaimage parallèle (§6). Générique, zéro nom de jeu en dur.

## 4. `Vigie` — contrôleur pur (repris de la meilleure idée d'third-party)

Side-effect free. Le `cycle` décide quoi faire du `Signal`.

```rust
enum Signal { Laisser, Avertir(String), Bloquer(String), Poser(String) }

impl Vigie {
    fn avant_appel(&mut self, appel: &Appel) -> Signal;   // même (nom+args) hashé répété → Bloquer
    fn apres_appel(&mut self, appel: &Appel, ok: bool, hash_resultat: u64) -> Signal;
    // détecte : échec exact répété, même outil échoue N×, idempotent sans progrès (même résultat N×)
}
```

Seuils **configurables** + **calibrés par modèle** (§9). Avertissements ON par défaut (injectés
comme guidance dans l'observation) ; arrêts durs opt-in. Remplace les `HashMap` inline du brain.

## 5. Longévité — `Jauge` + `Escale` (un seul système de contexte)

- **`Jauge`** : tokens **réels** depuis `usage` provider (Claude/OpenAI/DeepSeek/Codex). Fallback
  estimatif (tokenizer approx) seulement pour Ollama. Pilote *tout* : warnings, compaction, fatigue.
- **`Escale`** (entre passes, jamais en plein milieu) décide **une** action :
  - `Jauge < 70%` → rien.
  - `70–85%` → **compaction** : résume les vieux tours, **externalise** les gros résultats d'outils
    sur disque (garde un aperçu + chemin), conserve N derniers tours intacts. Préfixe stable préservé.
  - `> 85%` **ou** fatigue cognitive haute → **consolidation** : extrait les faits durables vers la
    mémoire graphe, écrit un **checkpoint** (`tasks.checkpoints.<id>`), repart sur contexte frais
    (~500 tk) + `mission` + rappel « relis ta mémoire ». (Ton mécanisme `fatigue` actuel, unifié ici.)
- **Anti context-rot** : contexte injecté **just-in-time** via `Source` (on récupère le souvenir
  pertinent au tour où il sert, en *trailing* pour ne pas casser le cache), pas tout en tête.

## 6. Sous-agents `Eclaireuse` — orchestrator-worker (pattern multi-agent Anthropic)

Pourquoi : isoler le contexte. Une recherche large pollue la fenêtre du parent ; on la confie à une
éclaireuse au **contexte propre**, qui ne remonte qu'un **rapport compact**.

```rust
enum Role { Eclaireuse, Ouvriere, Gardienne, Architecte }  // scout / exécutante / critique / synthèse
struct OrdreEclaireuse { role: Role, tache: String, contexte: Option<String>, plafond: usize }
struct Rapport { tache: String, synthese: String, faits: Vec<Fait>, sources: Vec<String> }
```

- **Budget séparé** (parent ≠ éclaireuse) : le total peut dépasser le plafond parent (idée third-party
  `IterationBudget`). Éclaireuses de recherche : 25–50 passes (vs 8 aujourd'hui — trop court).
- **Streamé** : le `tx` de l'éclaireuse est **branché** au parent (préfixe `eclaireuse:<id>`) →
  observabilité dans le dashboard (aujourd'hui jeté `_rx`).
- **Mémoire** : l'éclaireuse écrit ses découvertes dans le graphe → le parent les `memory_search`.
- **Essaimage parallèle** : `essaimage(vec![ordre1..N])` lance plusieurs éclaireuses en // (borné
  par `Semaphore`), chacune sur un axe distinct → recherche large *rapide*. La **Gardienne**
  (critique) vérifie les rapports avant synthèse (verify pattern).

## 7. Reprise & robustesse — `Carnet` + `Meteo`

- **`Carnet` sérialisable** (JSON dans `sessions/<id>.carnet.json`), `sauver()` à chaque fin de passe :
  itinéraire, `passe`, compteurs Vigie, Jauge, météo, dernier checkpoint. **Crash → reprise** exacte,
  pas re-départ à zéro. (Aujourd'hui tout est en RAM.)
- **`Meteo`** classe chaque erreur provider et applique la politique :

| Classe | Politique |
|---|---|
| `RateLimited{reset_at}` | sleep `Retry-After` puis reprise ; sinon **rotation de clé** (`credential_pool`) ; sinon déroutement modèle |
| `ReloginRequired` | stop net + message UI (pas de déroutement inutile) ; clé marquée invalide |
| `Transient` (5xx, reset, timeout réseau) | retry backoff exponentiel borné, **même** modèle |
| `Troncature` (`length`) | reprise « continue exactement » (borné), puis déroutement |
| `OutilTimeout` | observation d'échec + conseil (réduire l'action / `submit_job`), la boucle continue |
| `Fatal` | déroutement modèle (fallbacks) ; si tous échouent → `FinDeVol::Erreur` |

- **`stop_reason` propagé fidèlement** par chaque provider (corrige le `Some("stop")` codé en dur
  pour Anthropic/Ollama qui casse la détection de troncature).
- **Approbation** : timeout → comportement **configurable** (`auto` en mode autonome, `refuser` en
  mode surveillé) — plus d'auto-approve silencieux par défaut.

## 8. Outils, skills, plugins, mémoire — ce qu'on garde / améliore

Garde (bon design) : registry `Abeille`, sélection sémantique (`SEMANTIC_CORE` + index de capacités
cacheable), skills OKF en mémoire, `reload_plugins`, externalisation des gros résultats.
Améliore :
- **Partition réactivée** (§le bug `keep_single_tool_call`) : garder N appels si **tous** valides
  **et** concurrency-safe ; sinon retomber à 1. Read-only en // borné, mutating en séquentiel.
- **`memoire.db` 155 Mo** : profiler par table, purger l'index outils régénérable, viser un **seul**
  backend de prod (aujourd'hui sidecar/native/sqlite cohabitent). Sortir `search()` du chemin
  critique quand non pertinent (Source optionnelle, appelée à la demande).
- **Gros résultats d'outils** : plafond par outil + résumé extractif/LLM auxiliaire (déjà amorcé),
  toujours externalisés à la compaction.

## 9. Calibrage par modèle (local faible ↔ cloud fort)

`Ruche` porte un `ProfilModele` qui ajuste les rails selon la cible :

| Profil | Modèles | Rails |
|---|---|---|
| **Fragile** | gemma e4b/12b, qwen petit | tool_calls **1/tour** par défaut, fallback texte `<tool_call>` actif, Vigie stricte, nudges anglais explicites, déroutement rapide |
| **Robuste** | gemma 27/35b, qwen 32b, DeepSeek | partition ∥ activée, on fait confiance aux `stop_reason`, Vigie souple |
| **Natif outils** | Claude API, Codex | tools natifs API, `stop_reason` natif fait foi, heuristiques quasi off, parallélisme plein |

Un seul moteur, comportement piloté par données — pas de branches dupliquées.

## 10. Prompts en **anglais** (best practice confirmée)

Prompts internes (système, nudges, guardrail, extraction) **en anglais, en dur** : meilleur suivi
d'instructions (surtout modèles faibles), préfixe stable cacheable. Le modèle **répond dans la langue
de l'utilisateur** automatiquement — aucune instruction de langue nécessaire. Les strings **UI**
(`ChatEvent::Status`, labels) passent par une couche **i18n** séparée (fr/en/…). Prompt en **3 tiers** :
`stable` (identité, outils, garde) · `contexte` (cwd, fichiers projet) · `volatile` (mémoire, horodatage).

## 11. Migration — cohabitation, zéro projet cassé

1. Branche `git checkout -b butinage`.
2. Nouveau crate `laruche-butinage` ; l'ancien `brain.rs` reste intact.
3. `lib.rs` expose des **adaptateurs de compat** : `boucle_react_memoire(...)` et
   `boucle_react_memoire_multimodal(...)` deviennent de fines façades qui appellent `butiner()`.
   → le node ne change pas (contrat préservé) ; bascule par flag `RUCHE_MOTEUR=butinage`.
4. Tests d'or sur tes vraies missions (recherche longue, cron) : v1 vs v2.
5. Bascule, puis suppression de `brain.rs`.

Ordre d'implémentation : `issue`/`carnet`/`itineraire` → `cap/vigie`+`cap/boussole` (cœur testable)
→ `meteo` → `recolte` → `escale`+`jauge` → `eclaireuse` → `prompt` → adaptateurs → flag.

## 12. Best practices agentiques intégrées (référentiel 2025-2026)

- Boucle minimale pilotée par `stop_reason` (third-party `agent-loop.ts`).
- Contrôleur anti-boucle pur (third-party `tool_guardrails.py`).
- Budget d'itérations séparé parent/sous-agent (third-party `iteration_budget.py`).
- Tokens réels, prompt 3 tiers cacheable (third-party `system_prompt.py`).
- Partition read-only ∥ / mutating séquentiel (Claude Code `partitionToolCalls`).
- Orchestrator-worker + isolation de contexte des sous-agents (Anthropic « multi-agent research »).
- Context engineering : just-in-time retrieval, compaction, externalisation (Anthropic « effective context engineering »).
- Durable state / checkpoint pour reprise (Carnet).
- Plan/todo comme source de vérité de la terminaison (Claude Code).
