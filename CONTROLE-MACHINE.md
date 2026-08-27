# Contrôle de la machine

Deux choses dans ce document : ce que l'agent peut faire du PC aujourd'hui, tel que le code
l'implémente, et la conception de l'outil `computer` natif.

État : la partie 2 est construite. L'outil natif remplace le serveur MCP Python, l'arbre
d'accessibilité est en place sur Windows, et le halo est à l'écran. Ce qui reste est listé
en fin de document.

## Partie 1 : l'état des lieux

### Exécution directe, sans sandbox par défaut

Les outils natifs tournent dans le processus du nœud, donc avec les droits de l'utilisateur
courant. Aucun n'est isolé sauf demande explicite.

| Outil | Ce qu'il fait | Fichier |
|---|---|---|
| `shell_exec` | PowerShell `-NoProfile -NonInteractive` sur Windows, `sh -c` ailleurs, timeout 300s | `laruche-essaim/src/abeilles/shell.rs` |
| `execute_code` | `python -I -` sur stdin, 30s | `laruche-essaim/src/abeilles/execute_code.rs` |
| `run_script` | Enchaîne plusieurs outils en un tour, sans repasser par le LLM | `laruche-essaim/src/abeilles/run_script.rs` |
| `file_read/write/edit/search` | Pas de sandbox de chemin dans l'outil, le cadrage vient du moteur de permissions | `laruche-essaim/src/abeilles/fichiers.rs` |
| `plugin_create` | Écrit un manifeste dont le champ `command` est un template shell exécuté ensuite | `laruche-essaim/src/abeilles/forge.rs` |
| `mcp_add` | Inscrit `{command, args}` dans `mcp_servers.json`, lancé au prochain démarrage | `laruche-essaim/src/abeilles/forge.rs` |

Les deux derniers sont de l'exécution arbitraire différée : ils ne lancent rien tout de
suite, ils inscrivent quelque chose qui sera lancé plus tard. C'est à traiter comme du
`shell_exec`, pas comme de l'écriture de fichier.

Le sandbox Docker existe mais reste opt-in : `ESSAIM_SANDBOX_DOCKER=1` et docker présent,
sinon exécution nue sur l'hôte.

### Contrôle du navigateur

`browser`, dans `laruche-essaim/src/abeilles/navigateur.rs`. Actions : `navigate`, `read`,
`find`, `click`, `fill`, `key`, `hover`, `scroll`, `wait`, `eval`, `screenshot`, `console`,
`network`, `back`, `forward`, `tabs`, `select`, `close`.

Trois transports derrière une seule abstraction `Canal` :

- l'extension de `extension-chrome/` (permissions `debugger`, `tabs`, `host_permissions:
  <all_urls>`), donc le Chrome de l'utilisateur avec ses sessions déjà connectées ;
- un Chrome lancé par LaRuche sur profil persistant, piloté en CDP ;
- un Chrome existant démarré avec `--remote-debugging-port`.

C'est le vecteur le plus sensible en pratique, parce qu'il hérite des sessions
authentifiées. `eval` exécute du JS arbitraire dans la page, et `key` passe par le domaine
`Input` du protocole DevTools, donc produit des événements indistinguables d'une frappe
humaine.

### Souris, clavier, écran

Natif, outil `computer`, dans `laruche-essaim/src/abeilles/ordinateur.rs`. Le serveur MCP
Python `mcp/computer_use.py` a été supprimé, il n'avait plus de raison d'être. Et un serveur
MCP ne peut plus reprendre le nom d'un outil natif: le registre refuse et le journalise,
alors qu'il écrasait silencieusement jusqu'ici.

Deux chemins. Par l'arbre d'accessibilité : `windows`, `focus_window`, `read`, puis `click`
et `fill` sur un numéro de contrôle. Par les pixels : `screens`, `screenshot`, puis
`mouse_move`, les clics, `left_click_drag`, `scroll`. Plus `type`, `key`, `key_down`,
`key_up` pour le clavier.

### Déclenchement sans humain devant l'écran

- Watchers (`laruche-watchers/src/lib.rs`) : `Action::Commande { commande }` exécute une
  commande quand la règle se déclenche. Le watcher de type `Commande` en exécute une aussi
  comme observation.
- `cron_create`, `mission_create` avec cadence cron, `run_now`, `submit_job`, dans
  `laruche-node/src/abeilles_local.rs`.
- Entrées distantes : nœud sur `127.0.0.1:8419`, ou tout le LAN si `LARUCHE_BIND_LAN=1` ;
  surface MCP HTTP qui expose tout le registre, `shell_exec` et `file_write` compris,
  protégée par token plus allowlist IP (`laruche-node/src/mcp_pare_feu.rs`) ; canaux
  Telegram, Discord, Slack ; mesh `mesh_send` entre ruches.

### Ce qui filtre, et où ça fuit

Ordre réel dans `laruche-essaim/src/butinage_pont.rs` : garde anti-injection, règles de
refus utilisateur (plancher dur, non contournable même en mode auto), moteur de permissions
(`Default`, `Plan`, `AcceptEdits`, `Auto`, `Bubble`), puis smart approvals : déjà approuvé,
sinon juge LLM, sinon popup humain avec timeout 180s qui refuse par défaut. Les hooks
`pre_tool` peuvent bloquer en amont (`hooks.json.example`).

Trois points à garder en tête si on évalue la surface :

1. `PermissionMode::Auto` autorise tout ce qui n'est pas explicitement refusé.
2. Quand aucun humain n'est joignable (cron, watcher, éclaireuse) et que
   `approbation_stricte` est faux, `decider()` autorise l'appel non résolu. C'est le chemin
   par lequel une commande part sans validation.
3. `BLOCKED_PATTERNS` dans `shell.rs` est un `contains` sur la commande en minuscules.
   `Remove-Item -Recurse -Force C:\` ne matche aucun motif de la liste. La vraie barrière
   est le juge LLM plus le popup, pas cette liste.

## Partie 2 : l'outil `computer` natif, tel qu'il est construit

### Pourquoi built-in plutôt que MCP

Quatre raisons qui tiennent au code existant, pas à l'élégance.

**Le gating devient correct.** Tous les outils MCP renvoient `NiveauDanger::NeedsApproval`
en bloc (`mcp_tool.rs:29`) : un `screenshot` en lecture seule passe par la même porte qu'un
`left_click`. Pire, `cle_pattern` (`approbation.rs:142`) ne découpe finement que pour
`shell_exec` ; pour tout le reste la clé est `outil:<nom>`. L'utilisateur approuve une fois
un screenshot, et cette approbation couvre ensuite tous les clics, la frappe clavier et les
drags, pour toujours. En natif : `screenshot` et `cursor_position` en `Safe`, le reste en
`NeedsApproval`, et `cle_pattern` étendu pour keyer sur l'action, exactement comme il keye
déjà sur le binaire et la sous-commande shell.

**Le boot.** Le ROADMAP ligne 161 note les 8s d'imports Python mesurés pour ce serveur. Un
actionneur Rust in-process, c'est zéro.

**La promesse produit.** "One Rust binary, fully local" tombe dès qu'il faut installer
Python et pyautogui pour la fonction la plus visible du produit.

**Le pipeline image existe déjà.** `ResultatAbeille::images` (`abeille.rs:85`) est ce
qu'utilise `browser screenshot`. Le MCP repasse par l'encodage MCP générique.

Et un défaut de correction, pas seulement d'architecture : pyautogui clique en coordonnées
logiques. Sur un écran à 150% de mise à l'échelle, ou sur un setup mixte 4K plus 1080p, les
clics partent à côté. C'est un bug qu'on ne diagnostique jamais depuis le modèle, qui croit
avoir mal visé.

### Architecture

Le point structurant : **séparer l'actionneur de l'overlay.**

L'actionneur (entrées plus capture) va dans `laruche-essaim`, in-process, async, comme
n'importe quelle abeille. L'overlay a besoin d'une boucle d'événements GUI sur le thread
principal, ce que macOS impose et qu'un nœud tokio ne peut pas offrir. L'overlay est donc un
processus séparé, un petit binaire `laruche-halo` que le nœud lance à la demande, piloté par
le websocket local. `laruche-node/src/ws_navigateur.rs` fait déjà exactement ce genre de
pont pour l'extension Chrome : le motif est là, il suffit de le reprendre.

Crates candidates (à revalider sur crates.io au moment de l'implémentation) :

- **Entrées** : `enigo`. Windows SendInput, macOS CGEvent, Linux XTEST. Pour l'injection au
  niveau scan-code (RDP, jeux, certains installateurs), descendre sur la crate `windows` et
  SendInput en direct. À prévoir dans l'abstraction dès le départ, pas après.
- **Capture multi-écran** : `xcap`. `Monitor::all()` donne nom, x, y, largeur, hauteur,
  `scale_factor`, `is_primary`. C'est littéralement l'API du "sélectionner l'écran".
- **Overlay** : Tauri 2, déjà dépendance de `laruche-bureau`. `transparent: true`,
  `decorations: false`, `alwaysOnTop`, `skipTaskbar`, `set_ignore_cursor_events(true)` pour
  le click-through. Le CSS de `SCRIPT_GLOW` dans `navigateur.rs` se réutilise quasi tel quel :
  cadre, badge, panneau flottant, anneau de pression, animation de frappe. Une seule identité
  visuelle pour les deux surfaces, c'est là qu'est le vrai gain.

### Le contrat de coordonnées

À figer avant d'écrire une ligne, parce que c'est ce qui casse en silence.

Le modèle parle toujours en pixels du screenshot qu'il vient de recevoir. L'outil fait seul
la conversion vers les coordonnées physiques du bureau virtuel. La capture est réduite à
environ 1280px de large avant l'envoi (un screenshot 4K brut coûte une fortune en tokens et
brouille la visée), le facteur est mémorisé, et il est réappliqué au clic. Chaque screenshot
renvoie en texte la liste des moniteurs avec leur géométrie, pour que le modèle puisse dire
"écran 2" sans deviner.

C'est exactement la logique `ref_N` du navigateur, transposée : le modèle ne manipule jamais
des coordonnées système, il manipule ce qu'il a sous les yeux.

### Halo et curseur

Un halo par moniteur, pas une fenêtre unique couvrant la bounding box du bureau virtuel :
les écrans ne forment pas toujours un rectangle et les DPI sont mixtes. Seul l'écran actif
s'allume, ce qui rend visible où LaRuche agit, pas seulement qu'elle agit.

Le curseur se dessine dans l'overlay. Pas `SetSystemCursor` : ça modifie un état utilisateur
global, et si le process crashe pendant une action l'utilisateur reste avec un curseur abeille
jusqu'au redémarrage. En plus c'est du code entièrement différent par OS. Un curseur dessiné
dans l'overlay, avec la traînée qui glisse vers le point d'action comme le fait déjà `moveTo`
dans `SCRIPT_GLOW`, se comporte pareil partout et disparaît si le process meurt.

Le panneau flottant du navigateur (déplaçable, semi-transparent, une ligne par action) se
transpose directement : c'est le même besoin, dire ce qui se passe là où ça se passe.

### La réalité par OS

Le langage n'est pas la difficulté. Rust gagne clairement sur les entrées, la capture et le
packaging ; pour l'overlay on écrit du code par OS quel que soit le langage, et Tauri en
absorbe l'essentiel. Ce qui coûte, c'est ailleurs.

- **Windows** : le cas facile. Prévoir quand même l'UAC : aucune injection possible vers une
  fenêtre élevée depuis un process non élevé, et le modèle ne comprendra pas pourquoi son
  clic ne fait rien. Le détecter et le dire.
- **macOS** : autorisations TCC Accessibility et Screen Recording, accordées par binaire
  signé. Un build de dev non signé se refait redemander l'autorisation à chaque rebuild.
  Surmontable, mais à ne pas découvrir à la fin.
- **Linux X11** : rien de particulier.
- **Linux Wayland** : l'injection synthétique est bloquée par design. Il faut passer par
  xdg-desktop-portal RemoteDesktop, qui affiche son propre prompt. C'est un backend à part,
  pas "Linux".

Tout ça derrière un feature cargo (`gui-control`), sinon `xcap` et `enigo` traînent des
dépendances X11 dans les builds serveur headless et dans la CI. Le workspace fait déjà ce
genre d'exclusion avec `default-members` qui laisse `outils/icones` de côté.

### Deux points de sécurité propres à cet outil

Ils comptent plus que le reste, parce que cet outil contourne toutes les barrières
construites ailleurs.

**Le clic sur sa propre popup d'approbation.** L'agent peut cliquer "Approuver" dans
l'interface web de LaRuche. Toute la chaîne de `butinage_pont.rs` devient décorative. Il faut
une règle dure dans l'outil : refuser toute action dont les coordonnées tombent dans une
fenêtre appartenant à LaRuche, vérifiable via le titre et le PID de la fenêtre sous le
curseur.

**Le coupe-circuit.** Le script actuel met `pyautogui.FAILSAFE = False`, il n'y a donc aucune
sortie de secours. Le natif doit avoir un hotkey global d'abandon, et céder la main dès que
l'utilisateur bouge physiquement la souris. Ça s'articule avec le halo : le halo dit "j'ai la
main", le hotkey la reprend.

### Ce qui est construit

1. **Actionneur et capture.** `enigo` pour les entrées, `xcap` pour la capture multi-écran et
   l'énumération des fenêtres, derrière le feature cargo `gui-control`, actif par défaut.
   Contrat de coordonnées et réduction de la capture à 1280px de large.
2. **Gating par action.** `cle_pattern` (`approbation.rs`) sépare désormais regarder et agir,
   pour `computer` comme pour `browser`. Approuver une capture n'approuve plus un clic.
3. **Garde-fous.** Refus d'agir sur une fenêtre de LaRuche, comparée par PID via
   `xcap::Window`. Main rendue dès que l'humain bouge la souris de plus de 30 pixels.
   Coupe-circuit global `LARUCHE_COMPUTER=0`.
4. **Arbre d'accessibilité**, `ordinateur_arbre.rs`, UI Automation sur Windows. C'est ce qui
   rend l'outil utilisable sans vision, et ce qui le distingue de tous les wrappers pyautogui.
   Mesure sur une fenêtre Electron réelle : 36s en lecture naïve, 0,37s avec un cache request,
   qui ramène tout l'arbre en un seul aller-retour COM.
5. **Halo**, `ordinateur_halo.rs`, quatre fenêtres superposées Win32. Quatre barres plutôt
   qu'une fenêtre plein écran : `UpdateLayeredWindow` reverse tout le bitmap à chaque image,
   soit 440 Mo/s en 2560x1440 contre 60 Ko pour des barres de six pixels. Panneau flottant
   déplaçable, anneau qui suit le curseur, flash après capture, glissement animé du curseur
   et frappe progressive.

### Ce qui reste

- **macOS et Linux pour l'arbre.** AX sur macOS, AT-SPI sur Linux. Les entrées et la capture
  y marchent déjà, seul le chemin `read` est absent.
- **Le halo hors Windows.** Même remarque : c'est du code par plateforme, et Tauri absorberait
  l'essentiel si on ne veut pas écrire trois fois le même dessin.
- **Le hotkey d'abandon.** La reprise de main sur mouvement de souris couvre le cas courant,
  mais un raccourci global reste la sortie de secours qu'on veut sous la main.
- **Le halo par moniteur.** Aujourd'hui le cadre se dessine sur l'écran capturé, un seul à la
  fois. Sur un montage à DPI mixtes, une fenêtre par moniteur serait plus juste.
