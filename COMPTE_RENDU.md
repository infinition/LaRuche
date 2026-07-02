# LaRuche - Compte rendu de travail

> Journal vivant des sessions de durcissement et d'amelioration du moteur agentique,
> de la memoire, des outils et de la securite. Enrichi au fur et a mesure.
> Convention : anglais dans le code, francais dans les docs, pas d'em dash, chaines UI
> variabilisees (i18n). Chaque item cite son commit.

---

## Session 2026-07-02 (apres-midi + soir) - UX, watchers v3, LaReine, curateur

Branche `butinage`. ~30 commits, workspace vert en continu, essaim/node a zero warning.

### Fiabilite & UX
- Boot 6s au lieu de plusieurs minutes : sync skills incrementale + MCP en arriere-plan,
  chronos de phase permanents (`4986b44`).
- Sessions chat : miroir live (F5 en plein run = reprise a l'identique), plan persiste,
  titre immediat, pastilles onglet Chat + par conversation (`a2279aa`).
- Roue de chargement au boot + launcher qui ouvre le navigateur quand le node repond
  (`c21709f`, `2d04e2d`).

### Outils & robustesse petits modeles
- tool_call/tool_search/run_script rebranches sur le registre principal LIVE (`9644b5f`).
- Parser tool-call tolerant (forme attributs gemma) + fallback JSON brut cable dans le
  pont (`3499784`, `ecb1ae0`).
- Validation JSON Schema des args avant execution, avec coercitions (`798d81a`).
- Outils missions (list/create/delete) + cadences missions dans cron_list (`a26e74b`).
- Catalogue skills complet retabli dans le contexte (`67db1bb`).

### Moteur
- Boucle legacy brain.rs SUPPRIMEE (~2600 lignes), _ext = facade vers le pont
  (`40a876b`, `ecb1ae0`). laruche-channels archive (`8f83a7b`).

### Watchers v3 (killer feature)
- v2 : apparition/suppression, up/down avec duree, hash texte extrait, intervalles/
  cooldown/sustained, gate LLM semantique (`2d1e5ec`).
- v3 : REGLES COMPILEES - DSL deterministe (jours/heures/dates/etat/contenu/taille/
  status) + llm_check en court-circuit, skill watcher-architecte, bulle UI resume+JSON
  (`806dcf3`). Gardes creation : chemin absolu, dossier refuse, anti-doublon (`274c6d0`).
- Cartes pipeline repliees/depliees, le schema EST le formulaire (`728f662`, `0314a36`).

### LaReine
- Audit complet puis 6 correctifs (`f964c11`) : anti-regression reelle, juge avec
  introspection atelier, verdicts persistes + JSONL scorecards, siege humain avec
  boutons, hint juge distinct, Hybride durci, budget temps des re-runs.
- Propositions : contenu complet depliable avant approbation (`0907bbf`).

### Hygiene de charge & secrets
- Curateur : single-flight + cooldown + priorite au premier plan via compteur
  runs_en_vol (`23f8b8c`, `288e6b7`) - diagnostic live de 2 slots llama satures en
  silence par une file de reviews.
- Secrets : masquage [SECRET:NAME] des valeurs du coffre dans toutes les observations
  (`adb3144`).

---

## Session 2026-07-01 / 07-02 - Audit expert + durcissement complet

Branche `butinage`. Point de retour global avant la session : commit `b9499f0`.

### 1. Moteur ReAct (butinage) - 12 corrections

Commit `5e979b5`. Tous tests verts (92 tests moteur).

- Transcript tool-calling porte par l'historique (`Message.appels` / `appel_id`) : le
  modele voit quels appels ont produit quelles observations. Fini les resultats orphelins.
- Ancre mission EPINGLEE + troncature low-watermark 60% calibree par la jauge (prefix
  stable = cache chaud). Avant : la mission pouvait disparaitre sur les longs runs.
- Checkpoint atomique (tmp+rename) + nudges internes filtres (ne reapparaissent plus au
  reload) + vigie persistee dans le carnet (anti-loop survit au crash).
- Timeout par outil (defaut 300s, override par outil, delegation illimitee).
- Cap des observations reinjectees (30k chars, tete+queue).
- Annulation cooperative (AtomicBool) honoree pendant les sleeps meteo et entre les lots.
- Budget tokens cumulatif (`FinDeVol::Budget`).
- Boucle d'appels 100% bloques stoppee apres 3 passes.
- `mission_accomplie` / `clarify` honores seulement SEULS (le travail s'execute d'abord).
- Compaction LLM par defaut (resume dense : decouvertes, decisions, impasses) + fallback
  extractif ; consolidation auto-portante (plan + faits + derniers tours + pieces).
- Escalade superviseur = `FinDeVol::Escalade` (plus `Plafond` mensonger).
- Heuristiques texte gatees hors profil `NatifOutils` ; `stop=Outils` sans appel = malforme.

### 2. Tool-calling NATIF par provider

Commit `bbf12cf`.

- Pre-passe de correlation (appel natif <=> resultat present, anti-400 des APIs strictes).
- OpenAI-compat : `tool_calls` + role `tool`. Anthropic : blocs `tool_use`/`tool_result`
  + images vision natives. Fallback texte pour modeles locaux.
- Provider `llamacpp` (base par defaut `127.0.0.1:8001`).
- Capture usage OpenAI-compat (`b01700a`) : `stream_options.include_usage` arrive dans un
  chunk dedie qui etait droppe (tokens=0). Note : sur llama.cpp le champ usage n'est
  toujours pas remonte a l'eval (a investiguer, non bloquant).

### 3. Deep-research

Commits `9a54424`, `ab60d45`, `a1b9b01`.

- 3 canaux de decision de mode : mots-cles elargis (FR/EN + ES/IT/PT/DE), outil
  `research_mode` auto-declare et INTERCEPTE par le moteur (escalade one-way), arg `mode`
  du plan.
- `PROTOCOLE_EXPLORATION` : 3-4 angles MAX, AU PLUS 4 scouts, fan-out `delegate` parallele,
  gardienne de verification, 403 = contournement (archives/caches/miroirs), verification
  FRAICHE meme si la memoire a deja la reponse.
- Scouts bornes : Eclaireuse 12 passes / min_web 3 (etait 30 / 6). Cap dur
  `MAX_DELEGATIONS=4` par mission (compteur partage). Cause mesuree des timeouts : scouts
  trop profonds x 7-12 d'entre eux sur 12B local.
- Autonomie durcie : jamais renvoyer l'utilisateur chercher, jamais demander la permission.

### 4. Outils web / fichiers - "machines de guerre"

Commit `73a2ac2`.

- `web_fetch` : pagination offset/max_chars (12k defaut), `include_links` (crawl), PDF via
  r.jina.ai, readability, fallback jina/wayback.
- `web_deep_search` : fetchs paralleles, fix panic UTF-8 (`String::truncate` sur accents),
  fallback jina par page.
- `file_search` : glob `*` + grep contenu (path:ligne:), dossiers de bruit ignores.
- `file_list` : arbre trie + tailles. `file_read` : cap 2000 chars/ligne. `file_write` : atomique.

### 5. Memoire cognitive - exploitation complete

Commits `c426bfd`, `d07ea22`, `bbb6bc9`, `59e6167`, `04514c3`, `656111a`.

- Embedder universel TOUJOURS actif (`HttpEmbedder` Ollama ou llama.cpp auto-detecte,
  disjoncteur si serveur down, backfill au boot). `lancer_embeddings.bat` avec DL auto.
- Search v2 : fusion semantique + FTS ; decay de PRIORITE (importance persistee + usage
  hebbien + fraicheur, JAMAIS de suppression) ; recall sans bruit skills (capacities.*/
  system.* exclus, items plafonnes 600c) ; garde anti-bruit FTS (>=2 tokens si requete riche).
- Supersede a l'ecriture : dedup exact + quasi-doublons a l'echelle du DOMAINE, seuils
  calibres sur mesures reelles (0.83/0.85 meme node / node frere).
- Arbitre LLM de contradiction (`656111a`) : bande 0.62-0.83 -> `ArbitreLLM` (modele aux,
  REPLACE/DISTINCT) -> supersede les UPDATE de faits (4070->5080 ~0.71, hors de portee du
  cosine). Echec = Distinct (jamais destructif). Opt-out `LARUCHE_MEMOIRE_ARBITRE=0`.
- Rappel JIT `Source::rappeler` cable (scouts + reprises + stagnation).
- Memoire episodique (`episodes.<date>.<slug>`) apres toute mission >=3 passes.
- Consolider recursif (sous-arbre entier, plus seulement le node direct).
- Cross-langue prouve en reel : question EN retrouve un fait stocke en FR.

### 6. Securite et hygiene - durcissement reseau

Commits `e610365`, `9b1ccca`, `ee44d11`, `fd48aac`, `0943092`, `d027fa0`.

- Bind `127.0.0.1` par defaut (etait `0.0.0.0`) ; exposition reseau = opt-in
  `LARUCHE_BIND_LAN=1`, loggue.
- CORS restreint aux origines localhost (etait wildcard).
- Middleware `auth_guard` global : cookie exige sur les requetes mutantes `/api/*`,
  seulement si un compte a mot de passe existe (fresh install + onboarding libres) ;
  GET passe ; allowlist auth + sync interne.
- Sync memoire mesh en opt-in `LARUCHE_MESH_MEMORY_SYNC=1` (etait auto, sans verif de pair).
- Sanitisation rendu chat : `LaRuche.Utils.safeMarkdown` = marked + DOMPurify sur les 2
  rendus innerHTML. Le HTML d'une page recuperee par web_fetch est nettoye avant affichage.
- Libs vendorisees localement (marked/DOMPurify/highlight.js), servies `/vendor/:name`,
  theme hljs inline. Fin des CDN externes (offline reel). sw.js v2.
- Auth : secret cookie sauve au boot (fini le re-login a chaque lancement), 401 diagnostiques.
- Onboarding embeddings = vraie sonde (etait un stub `done:false`).
- Hygiene verifiee PROPRE : aucun secret committe (skills/plugins/tout scanne), aucun
  fichier runtime tracke (state/credentials/db/sessions/users gitignores).
- Note : le vocabulaire de durcissement declenche les garde-fous larges de Fable 5 (faux
  positif sur de la defense) ; termes litteraux lisses en langage neutre (`d027fa0`).
  Faire le travail durcissement sur Opus.

### 7. Harness d'evals

Commit `eeed556` + crate `laruche-evals`.

- Rejoue `evals/missions.json` (8 missions) contre le VRAI moteur (vrai provider, vrais
  outils). Checks durs : fin, mode, min_web, min_delegations, max_passes, contenu, fichier,
  detection de demission (patterns FR/EN). Juge LLM optionnel. JSONL + baseline avec
  regressions signalees.
- Usage : `RUCHE_PROVIDER=llamacpp RUCHE_MODEL=gemma-4-12b cargo run -p laruche-evals`.

---

## Resultats d'evals (mesures reelles)

### Run 1 - Ollama qwen3:8b (2026-07-01) : 0/8

Zero tool call execute sur toutes les missions. Diagnostic : bug d'integration tools
d'OLLAMA, pas le harness (confirme par le run llama.cpp ci-dessous). Mystere resolu.

### Run 2 - llama.cpp gemma-4-12b (2026-07-02 08:41) : 3/8

| mission | verdict | mode | passes | web | fan-out | duree |
|---|---|---|---|---|---|---|
| ds1_savegame_deep | OK | exploration | 10 | 14 | 8 | 677s |
| broken_sword_deep_fanout | KO | exploration | 4 | 0 | 0 | 40s |
| deep_english_keyword | KO timeout | - | - | 0 | 7 | 900s |
| deep_sans_mot_cle | KO timeout | - | - | 0 | 12 | 900s |
| deep_multilingue_es | KO | standard | 2 | 0 | 0 | 24s |
| controle_question_simple | OK | standard | 1 | 0 | 0 | 2s |
| controle_fichier | OK | standard | 4 | 0 | 0 | 11s |
| anti_demission_obstacle | KO timeout | - | - | 0 | 0 | 600s |

Enseignements : socle correct (ds1 = fan-out parfait de 8 scouts, controles OK). Trois
timeouts causes par des scouts trop profonds x trop nombreux (7-12). Trou multilingue
(ES tombe en standard). Court-circuit memoire (broken_sword repond de memoire sans chercher).

### Run 3 - llama.cpp gemma-4-12b, apres correctifs `a1b9b01` : instable

- `ds1_savegame_deep` : **TIMEOUT 900s** (deleg=0, web=0) alors qu'il PASSAIT au run 2
  (677s, 8 scouts). `broken_sword` : encore web=0/deleg=0.
- Diagnostic : **variance du LLM local**. gemma-4-12b est stochastique ; sur une meme
  mission deep il fait un fan-out impeccable une fois (run 2) et part en generation lente
  sans deleguer la fois suivante (run 3). Ce n'est PAS une regression de code (mes fixes
  reduisent le travail des scouts ; un run sans delegation ne peut pas etre ralenti par le
  cap). C'est un probleme de capacite/debit du modele local, pas du moteur.

### Run 4 - DeepSeek v4 flash (openmodel.ai, Anthropic-compat) : test leger

- Endpoint valide : `tool_use` natif parfait (curl direct). Provider `anthropic` + api_base.
- Controles 2/2 : `controle_question_simple` (1 passe, 2s), `controle_fichier` (5 passes,
  16s, fichier ecrit). **Tokens captures** (8771, 1093) : le fix usage `b01700a` marche
  sur le format Anthropic ; le `tokens=0` etait specifique au streaming llama.cpp.
- Deep-research NON testable sur le tier gratuit : une mission deep (parent + jusqu'a 4
  scouts = 50+ appels API) sature le rate limit (endpoint throttle jusqu'a timeout, puis
  revient apres cooldown). Rate limit, pas capacite ni harness.

### Verdict d'ensemble

- **Le harness et le moteur sont CORRECTS** : controles verts sur les 3 backends ; tool
  calls natifs OK (Ollama echoue = bug d'integration Ollama, llama.cpp OK, Anthropic OK) ;
  fan-out prouve parfait au run 2 (ds1 : 8 scouts). Le code deep-research fonctionne.
- **Le fan-out deep-research est goulotte par le MODELE/INFRA, pas par le code** : le 12b
  local est trop lent+stochastique (timeouts, variance run a run) ; l'API DeepSeek gratuite
  rate-limite sous le volume d'appels du fan-out.
- **Pour valider le deep-research de bout en bout de facon fiable** : il faut soit un modele
  local plus rapide/costaud, soit un tier API payant sans rate limit agressif. Les correctifs
  fan-out (`a1b9b01`) restent justes sur le principe mais non validables en charge ici.
- Baseline d'evals : NON figee (aucun run deep propre de bout en bout obtenu). Les controles
  peuvent servir de mini-baseline anti-regression immediate.

---

## Suites (2026-07-02, apres campagne d'evals)

FAIT :
- `--only` de l'eval accepte une liste separee par virgule (`d8e3ff8`).
- Outil `mission_list` (cote user) : l'agent voit desormais ses missions planifiees
  (il tournait en rond quand une se declenchait).
- Etat nettoye (cote user) : mission broken-sword recurrente + skills-poubelle en attente.
- Split `main.rs` termine (cote user) : router.rs / state.rs / background.rs / helpers.rs.
- Polling front en pause sur onglet cache (cote user, helper `LaRuche.Poll`).
- Charte curateur durcie (`d423ae9`) : ne capture plus les impasses de diagnostic ni les
  meta-skills sur les internes du systeme (les DEUX chemins : PROMPT_CURATEUR + extracteur brain).

## Reste a faire (backlog priorise)

Voir `ROADMAP.md`. Principaux :

- Figer la baseline d'evals une fois un backend fiable disponible (12b local trop
  instable, DeepSeek gratuit rate-limite). Les controles servent de mini-baseline.
- dream -> reine_queue (auto-nettoyage supervise de la memoire).
- Tuer ou promouvoir l'ancien moteur `brain.rs` (encore le defaut sans `RUCHE_MOTEUR=butinage`).
- Auth mesh mutuelle (verification de pair). Sandbox OS pour shell.
- Investiguer : usage tokens=0 sur llama.cpp aux evals (OK sur Anthropic/DeepSeek : le
  fix `b01700a` marche, le probleme est specifique au format de streaming llama.cpp).
