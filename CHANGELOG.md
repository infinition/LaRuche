# Changelog

## [1.3.0] - 2026-08-27

Les deux outils de pilotage, `computer` et `browser`, passent de "ca marche sur les cas
simples" a "ca dit ce qu'il se passe quand ca ne marche pas". La plupart des corrections
ci-dessous portent sur des echecs SILENCIEUX: l'outil rapportait un succes et rien n'avait
eu lieu, ce qu'un modele n'a aucun moyen de detecter.

### Ajoute

- **Controle machine natif** (`computer`), en Rust, sans passer par Python: souris, clavier,
  capture multi-ecran, et surtout l'arbre d'accessibilite UI Automation, qui rend l'outil
  utilisable sans vision. Halo a l'ecran pour que l'humain voie ce qui est pilote.
- Pilotage des fenetres: deplacer, redimensionner, reduire, agrandir, restaurer, fermer.
  Les fenetres reduites sont enfin listees, et chaque fenetre dit sur quel ecran elle est.
- Detection de l'elevation (UAC). Windows filtre les entrees vers une fenetre administrateur
  en silence, sans erreur; c'est desormais annonce au lieu d'etre subi en boucle.
- Coupe-circuit clavier `Ctrl+Alt+Shift+H`, verifie a chaque caractere pendant une frappe.
- `mouse_down`/`mouse_up`, `triple_click`, `find`, `wait`, presse-papiers, `release_all`,
  et un relachement automatique de tout ce qui reste enfonce au bout d'une minute.
- **Navigateur**: pilotage du Chrome de l'utilisateur avec ses sessions ouvertes, panneau
  de page qui parle et auquel on peut repondre.
- Lecture de page a travers le shadow DOM et les iframes de meme origine.
- `overlays`: ce qui recouvre la page, bandeaux de consentement compris, avec les refs des
  boutons. Les bandeaux de consentement sont signales, jamais acceptes a la place de
  l'utilisateur.
- Vrais evenements souris: `right_click`, `double_click`, `middle_click`, `drag`.
- `upload`, `download`, `cookies` (noms et tailles, jamais les valeurs), `open_tab`,
  `dialog`, et `resize` avec emulation tactile pour verifier une mise en page responsive.
- **Table ronde**: constitution, pool de specialistes, moteur de debat, interface en direct
  avec verdict, historique et avatars.
- `web_discover`, qui trouve ce qu'un site ne lie pas, avec logs CT et plan du site.
- `web_fetch` gagne `focus` et `probe`: lire ce qu'on cherche, verifier une affirmation
  sans lire la page entiere.
- Retention des episodes: purge a la demande, ou age au-dela duquel ils s'effacent seuls.
  Le reglage part a zero, tout garder.
- Le seam d'empreinte TLS, en option et inerte par defaut, et une memoire des routes par
  lesquelles chaque hote se laisse lire.

### Corrige

- `computer`: un `ref` fait enfin ce qu'il annonce. `hover` cliquait, `middle_click` faisait
  un clic gauche, `scroll` et `left_click_drag` actionnaient l'element. Un curseur se faisait
  cliquer en son centre, donc regler au milieu de sa course.
- `browser`: `fill` sur un `<select>` levait systematiquement une exception, alors que les
  `<select>` recevaient bien un numero a la lecture.
- Les dialogues JavaScript figeaient la page jusqu'au timeout, sans que rien ne dise
  pourquoi.
- L'approbation: regarder et agir cessent de partager une approbation, et un outil atteint
  par `tool_call` est juge comme lui-meme.
- La garde en octets amputait l'agent sur un mauvais diagnostic; le mur des 80 Ko est reel.
- Cinq skills enseignaient des outils qui n'existent plus, `plan_mode` existait sans etre
  joignable, et l'index des capacites ne purgeait jamais les outils natifs retires.
- Le chat rend le markdown au fil de l'eau, le fil suit ce qui arrive sans empecher de
  lacher pour lire, et cent propositions en attente ne mangent plus la page.
- Un skill ajoute au depot arrive enfin jusqu'a l'agent au demarrage.

## [0.2.0] - 2026-04-05

### Added
- L'Essaim agent engine with ReAct loop
- 23+ built-in Abeilles (tools)
- Multi-provider LLM support (Ollama, OpenAI, Anthropic)
- Miel Protocol v0.2.0 (renamed from LAND)
- SPA unified dashboard + chat
- CLI TUI with Ratatui (WebSocket streaming)
- Telegram bot integrated in server
- RAG Knowledge Base with vector search
- Sub-agent delegation
- Parallel tool execution
- Browser control (headless Chrome)
- Dynamic plugin system
- Voice pipeline (STT/TTS)
- GPU/VRAM monitoring
- Interactive approval gating
- Cron scheduler
- MCP server support

### Changed
- LAND Protocol renamed to Miel Protocol
- All capabilities updated (Agent, Stt, Tts added)

## [0.1.0] - 2026-03-30

### Added
- Initial LaRuche node with LAND Protocol
- Ollama inference proxy
- mDNS discovery
- Basic dashboard
- CLI tool
- VS Code extension
