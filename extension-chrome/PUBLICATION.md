# Publication sur le Chrome Web Store

Etat au 1er septembre 2026: le manifeste et le paquet sont prepares pour MV3,
mais l'extension ne doit pas encore etre soumise. Le protocole de pilotage
contient un point bloquant de politique Web Store decrit ci-dessous.

## Point bloquant: code recu a l'execution

`background.js` recoit actuellement du JavaScript du noeud local dans les
actions `eval`, `glow` et `tap`, puis l'execute avec
`chrome.debugger` et `Runtime.evaluate`.

Pour le Chrome Web Store, du code execute par le navigateur et absent des
fichiers de l'extension est du code heberge a distance, meme quand il arrive par
une WebSocket locale. La documentation Chrome precise aussi que
`chrome.debugger` ne doit pas servir de remplacement general a
`chrome.scripting`. Une soumission en l'etat presente donc un risque eleve de
refus `Blue Argon`.

La correction attendue est architecturale:

1. embarquer dans l'extension le runtime qui lit, trouve, clique, remplit,
   dessine le halo et collecte console et reseau
2. remplacer les scripts transmis par le noeud par des commandes structurees et
   des donnees JSON
3. supprimer l'action generique `eval` de la variante Web Store
4. limiter les commandes DevTools a une liste fermee, comme c'est deja le cas
   pour l'action `cdp`
5. verifier qu'aucun texte recu par WebSocket n'est execute comme du code

Une diffusion privee, non repertoriee ou beta ne dispense pas de cette regle.

References officielles:

- [Deal with remote hosted code violations](https://developer.chrome.com/docs/extensions/develop/migrate/remote-hosted-code)
- [Improve extension security](https://developer.chrome.com/docs/extensions/develop/migrate/improve-security)
- [Publish an MV3 extension](https://developer.chrome.com/docs/extensions/develop/migrate/publish-mv3)

## Ce qui est deja conforme

- manifeste MV3 avec service worker
- description systeme en anglais, 107 caracteres sur 132 maximum
- anglais par defaut dans le popup, francais disponible au choix
- aucun CDN, aucune bibliotheque distante, aucune analytique
- pilotage, compagnon, curseur et enregistrement desactives par defaut
- `desktopCapture` retire car il n'etait pas utilise
- permissions de capture demandees seulement quand l'enregistrement est active
- acces aux sites et `scripting` demandes seulement pour le compagnon ou le
  curseur abeille
- navigation privee explicitement desactivee
- politique de confidentialite publique et liee depuis le popup
- valeurs de cookies retirees avant le pont vers le noeud local
- commandes CDP limitees a une liste fermee
- manifeste du ZIP Web Store place a la racine par le workflow de release

## Identifiant de l'extension

Le paquet de developpement garde le champ `key` pour produire l'identifiant
`ahgfjacmpohglimmcfnlbeccdghpkboo`. Le workflow retire ce champ du paquet Web
Store.

Lors de la premiere soumission brouillon:

1. televerser le ZIP sans publier
2. relever l'Item ID attribue par le tableau de bord
3. ajouter cet ID a la liste acceptee par le noeud, ou le fournir dans
   `LARUCHE_EXTENSION_ID`
4. verifier la connexion avec la version installee depuis le Web Store
5. ne publier qu'apres ce test

Sans cette etape, la version Web Store sera installee mais le noeud refusera sa
WebSocket car son identifiant sera different de celui du build de developpement.

## Texte de fiche propose

### Nom

LaRuche

### Resume

Lets your hive drive this browser: it opens its own tabs in a visible group and
marks the page it controls.

### Objectif unique

Connect Chrome to a LaRuche node running on the same computer so the user can
let the agent operate selected browser tabs visibly and stop it at any time.

### Description

LaRuche connects Chrome to the LaRuche node running on your computer. It lets
the browser agent use your current Chrome session, including tabs you explicitly
select, while keeping every automated action visible.

Agent tabs are placed in a dedicated yellow group. Chrome shows its debugger
banner while a tab is controlled, and LaRuche adds a visible border, status
panel and optional bee cursor. Turning off Allow control immediately closes the
local connection and detaches the debugger.

The popup can also save the current page or a note to local LaRuche memory. An
optional recorder captures the active tab, a selected Chrome window or a screen
and saves the video through Chrome. Manual recording works without an active
agent.

The extension connects only to the LaRuche node on `127.0.0.1`. It contains no
advertising, analytics or developer-operated collection service. LaRuche is
required and is available separately from the project repository.

## Justification des permissions

| Permission | Justification a saisir |
|---|---|
| `activeTab` | Read the page selected when the user opens the popup and authorize a manual active-tab recording. |
| `alarms` | Wake the MV3 service worker for bounded reconnection, pending local notes and automatic control timeout. |
| `debugger` | Core purpose: inspect, navigate, capture and interact with the one tab visibly controlled by LaRuche. Chrome keeps its debugger banner visible. |
| `storage` | Store language, local node port, feature settings, companion position and a bounded queue of unsent local notes. |
| `tabGroups` | Put controlled tabs in a visible yellow LaRuche group and restore a borrowed tab to its previous group. |
| `tabs` | List open tabs so the user and agent can select one, and read the selected tab title and address for the Keep feature. |
| `downloads`, facultatif | Save a recording to the user's Downloads folder or selected location. |
| `offscreen`, facultatif | Keep MediaRecorder running after the popup closes because MV3 service workers have no DOM or media encoder. |
| `tabCapture`, facultatif | Record the active tab only after an explicit popup action. |
| `scripting`, facultatif | Install the optional companion and bee cursor only after the user enables one of them. |
| `http://*/*`, `https://*/*`, facultatif | Display the optional companion or bee cursor on websites after a separate site-access grant. |

## Confidentialite dans le tableau de bord

URL de politique:

`https://github.com/infinition/LaRuche/blob/main/extension-chrome/PRIVACY.md`

Les declarations doivent au minimum couvrir le contenu des sites, l'historique
de navigation represente par les adresses d'onglets, l'activite utilisateur et
les donnees de formulaire traitees pendant le pilotage. L'acces aux metadonnees
de cookies doit aussi etre declare avec prudence: les valeurs sont retirees dans
l'extension avant le pont local, mais la reponse DevTools les contient
temporairement.

Certifications a confirmer dans le tableau de bord:

- utilisation limitee a l'objectif unique et aux fonctions visibles
- aucun usage publicitaire ou de profilage
- aucune vente de donnees
- aucun usage pour le credit
- aucun humain ne lit les donnees via un service du developpeur
- transfert uniquement quand il est necessaire a un fournisseur choisi et
  configure par l'utilisateur dans son noeud LaRuche

Les reponses du tableau de bord, la fiche et `PRIVACY.md` doivent rester
strictement coherentes.

## Verification avant envoi

1. charger le dossier non empaquete et tester toutes les fonctions
2. verifier le bouton d'enregistrement manuel sans agent connecte
3. tester le refus puis l'acceptation de chaque permission facultative
4. tester l'arret du pilotage et le retour d'un onglet emprunte dans son groupe
5. executer les controles JavaScript, JSON et Rust
6. generer le paquet Web Store avec le workflow de release
7. verifier que `manifest.json` est a la racine du ZIP et que `key` en est absent
8. televerser en brouillon et tester l'Item ID du Web Store avec le noeud
9. fournir captures, icone, categorie, email de support et politique de
   confidentialite dans le tableau de bord
10. utiliser la publication differee pour garder la main apres validation
