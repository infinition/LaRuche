# Changelog

## [1.4.0] - 2026-09-01

Une version de surface: la page de presentation, le wiki et l'extension. Le moteur ne bouge
presque pas, mais trois pannes de l'interface se ressemblaient toutes: le code rapportait un
etat qui n'etait pas le vrai, et l'affichage remplacait la cause par une phrase toute faite.
Le kanban, la verification de mise a jour et le glisser-deposer des cartes viennent de la.

### Ajoute

- **Compagnon abeille** dans l'extension Chrome: eteint par defaut, allume depuis le popup,
  present sur les pages visitees et retrouve la ou il etait quand on change d'onglet.
- **Page de presentation** refaite: quatorze maquettes des vraies interfaces (table ronde,
  pilotage du navigateur et du bureau, mode vocal, memoire, flux, Telegram, essaim, messagerie
  entre ruches, approbation, LaReine, watchers, capacites), un carrousel qui les fait defiler,
  et une abeille qui vit sur les deux pages: elle se pose, cligne, s'endort et reve.
- **Navigation douce** entre l'accueil et le wiki, sans rechargement.
- **Interrupteur de l'abeille** dans la barre du haut du site, retenu d'une visite a l'autre.
- **Kanban**: un bouton Lancer sur les cartes qui attendent, et le choix de la colonne des la
  creation. Une tache naissait toujours dans A faire, et seule Pret est relevee.
- Onglet **Aide** dans les reglages: version installee, verification des mises a jour, liens
  vers le wiki et le depot.
- Releve du kanban reglable, panneau mesh lisible, et une reine qu'on distingue du reste.
- **Camera**: une image de la webcam, rendue au modele.
- Un backend TTS generique, au lieu d'un par moteur.
- L'extension garde un lien ou une note dans la memoire, en un clic.
- Le transcript porte le raisonnement des modeles thinking.
- La **constitution** de la table ronde a son icone, son titre et son editeur dans l'arbre de
  la memoire, comme les autres prompts systeme. Elle s'ouvrait en noeud ordinaire, et son
  editeur aurait propose d'enregistrer le texte de l'identite a sa place.

### Corrige

- **Kanban, edition d'une tache**: les champs s'ouvraient vides. Le code relisait la SIGNATURE
  de comparaison du tableau comme si c'etait la liste des taches; l'exception etait avalee par
  un `catch` vide, et enregistrer aurait ecrase la tache avec du vide.
- **Kanban, glisser-deposer**: les cartes ne se deplacaient pas. Le glisser-deposer HTML5 ne
  se declenche pas dans la fenetre de l'application et n'existe pas au doigt; le deplacement
  est desormais suivi au pointeur, avec la colonne visee mise en evidence.
- **Verification de mise a jour**: elle annoncait "impossible de joindre GitHub" alors que le
  noeud repondait et que GitHub aussi. C'etait une faute de code dans l'affichage du resultat,
  attrapee par le filet qui remplacait toute panne par cette phrase. Elle ne se declenchait
  que lorsqu'une mise a jour existait vraiment. Le filet dit maintenant ce qu'il a attrape.
- Les liens externes de l'aide s'ouvrent dans l'application.
- Images: mises au gabarit avant l'envoi, refus d'image reconnu sous son deguisement et
  verifie avant de conclure, l'image reste visible aux tours suivants, et enregistrer un
  profil n'efface plus sa cle.
- Le micro ne s'ouvre plus au milieu de sa propre phrase.
- Le repli sur la voix du navigateur cesse d'etre silencieux.
- Une image rendue par un outil arrive enfin jusqu'au modele, apres l'observation.
- Coller une image marche, et un fichier lache sur la zone d'envoi aussi.
- Le halo anime debordait de sa pastille dans les maquettes de la page de presentation.
- **La version affichee**. Les paquets sont restes en 0.2.0 pendant que les releases
  partaient en v1.x: l'onglet Aide annoncait v0.2.0 et se croyait en retard de trois
  versions sur lui-meme. L'espace de travail passe en 1.4.0, la coque de bureau aussi.
- **Table ronde**: un debat se supprime, et l'historique s'ouvre et se cherche comme
  celui des conversations. La route de suppression existait cote noeud depuis le debut,
  l'interface n'en offrait simplement pas le chemin.
- **La constitution de la table ronde est en anglais**, comme les autres prompts systeme.
- **Les capacites MCP disparues**. Un serveur devenu injoignable laissait ses outils dans
  l'arbre de la memoire pour toujours: le balayage ne passait que si le registre
  contenait au moins un outil MCP, et zero serveur joignable veut dire zero outil. La
  memoire annoncait des capacites que le noeud n'avait plus, `capacities.mcp.computer`
  en tete, longtemps apres la suppression du serveur Python qu'il decrivait.
- **La CI**. Un test verifiait le message d'une action inconnue, mais l'outil enumerait
  les ecrans avant de lire le nom de l'action: sur une machine sans affichage il
  echouait sur les moniteurs. Le nom est verifie d'abord, ce qui est aussi la bonne
  reponse a donner a une faute de frappe. Deux avertissements clippy corriges avec.
- **Releve de la colonne A faire** (Missions / Kanban): a l'echeance reglee, les taches
  de A faire passent dans Pret, les plus anciennes d'abord, et la releve de Pret les
  execute une par une avec le fournisseur de chacune. Elle ne lance rien elle-meme:
  promouvoir plutot qu'executer garde un seul chemin d'execution, donc une seule facon
  de se tromper. Eteinte par defaut, cadence en heures, jours ou semaines, et un bouton
  pour relever tout de suite.
- Les releases ne sortent plus en brouillon: il fallait venir cliquer Publish a la main
  apres chaque tag, sans que rien ne le dise.

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
