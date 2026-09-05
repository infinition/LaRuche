# DS Studio

Carnets d'analyse de donnees pour le bac a sable LaRuche, pilotables par un
agent. L'agent ecrit les cellules, les execute une par une, et lit les tableaux
et les graphiques produits. Le carnet suit l'agent pendant qu'il travaille, et
rend la main des que vous remontez le fil.

Portage de `obsidian-python-ds-studio` sous les contraintes du bac a sable
LaRuche, plus strictes que celles d'Obsidian et qui changent l'architecture.

## Ce que le bac a sable impose

Verifie dans le code de l'hote, pas suppose :

| Contrainte | Source | Consequence sur l'app |
|---|---|---|
| iframe `sandbox` sans `allow-same-origin` | `apps.js:677` | Origine opaque : `localStorage` et IndexedDB levent une exception. Toute persistance passe par `storage.private`. |
| CSP `default-src 'none'`, pas de `worker-src` | `assets.rs:104` | Web Workers bloques. Le decoupage worker du plugin Obsidian n'est pas portable : le noyau tourne sur le thread principal. |
| CSP `script-src` sans `'unsafe-eval'` | `assets.rs:104` | Ni `eval` ni `new Function`. Le langage du carnet est tokenise, parse et interprete a la main. |
| CSP `connect-src` limite aux assets du paquet | `assets.rs:104` | L'app ne peut rien telecharger. L'agent, lui, le peut : il recupere le fichier avec ses propres outils et le passe a `data.add`. |
| Archive 32 Mio compressee | `installer.rs:10` | Pyodide doit tenir dans ce budget. Le coeur plus numpy et pandas passe, matplotlib rarement. |
| Handler d'action : 15 s | `Developing-Apps.md:227` | `cell.run` ne bloque pas : elle rend un `jobId`, l'agent interroge `job.status`. |
| Message de pont : 64 Kio, resultat 60 Kio | `Developing-Apps.md:265` | Chaque reponse est construite contre un budget d'octets qui tronque et le signale. |
| `storage.private` : 1 Mio, 64 Kio par valeur, 256 cles | `storage.rs:15-18` | Carnets decoupes en morceaux, budget de resultats, jeux de donnees plafonnes. |

## Les deux noyaux

Une seule interface de noyau. Ce qui repond depend de ce qui est embarque dans
le paquet au moment du build.

**Noyau integre, par defaut.** Un langage de carnet interprete dans la page,
sans dependance, pret en une image. La syntaxe est calquee sur pandas parce que
c'est ce qu'un modele ecrit le mieux :

```
v = load("ventes")
top = v.filter(annee >= 2020).groupby("produit").agg(total = sum(montant), n = count())
show(top.sort("total", desc = true).head(10))
bar(top, x = "produit", y = "total", title = "Ventes par produit", color = "#ccff00")
```

Verbes : `filter`, `select`, `drop`, `rename`, `sort`, `head`, `tail`, `slice`,
`assign`, `withColumn`, `groupby`, `agg`, `pivot`, `join`, `describe`,
`distinct`, `dropna`, `fillna`, `valueCounts`, `unique`, `histogram`, `concat`.
Agregations : `sum`, `mean`, `min`, `max`, `count`, `countDistinct`, `median`,
`quantile`, `std`, `variance`, `first`, `last`, `mode`. Sorties : `show`,
`print`, `md`, `save`, `bar`, `line`, `area`, `scatter`, `pie`, `hist`.

**Noyau Python, experimental.** CPython en WebAssembly via Pyodide, avec
pandas et numpy. Meme vocabulaire de sortie, et les figures matplotlib sont
capturees en PNG. Il faut l'embarquer avant le build :

```powershell
python examples/apps/ds-studio/tools/vendor_pyodide.py
python examples/apps/ds-studio/build.py
```

L'app detecte le runtime au demarrage et bascule. Si Python est embarque mais ne
demarre pas, elle le dit et repasse au noyau integre plutot que d'executer
silencieusement un autre langage que celui dans lequel le carnet est ecrit.

## Le contrat agent

`kernel.status` renvoie `language` et une reference complete : verbes,
agregations, appels de graphiques, exemples, et ce qui n'existe pas. Chaque
`notebook.state` en rappelle une version courte. L'agent n'a donc jamais a
deviner la syntaxe ni a aller lire le paquet.

Ordre attendu, repete dans le guide du manifeste :

1. `app_wait` jusqu'a **Pret**. Le handshake SDK ne signifie que le transport
   est ouvert. L'hote rejette les actions avant.
2. `kernel.status` une fois, pour le langage et la reference.
3. `data.list` et `data.preview` avant d'ecrire quoi que ce soit.
4. `notebook.state` pour `revision`.
5. `cell.add` puis `cell.run`, qui rend un `jobId` immediatement.
6. `job.status` en boucle jusqu'a `ok`, `error` ou `cancelled`.

Toute action mutante exige la revision exacte et est rejetee si elle est
perimee ; le message donne la revision courante. `cell.update` accepte
`mode: "append"` : l'agent envoie le code par fragments et l'utilisateur voit la
cellule s'ecrire.

Vingt-cinq actions declarees, guide detaille sous les 16 Kio autorises. Les
modeles inventent regulierement des noms d'arguments, alors `id` est accepte
pour `cellId` et `notebookId`, `content` pour `source`, et un champ `language`
est accepte puis ignore. Un argument vraiment absent produit un message qui
nomme celui qui manquait.

## Carnets

Les carnets sont enregistres automatiquement, avec leur nom, leurs cellules et
leurs resultats. `notebook.list`, `notebook.open`, `notebook.new`,
`notebook.rename`, `notebook.delete` et `notebook.clear` couvrent la
bibliotheque cote agent ; le selecteur en haut a droite fait la meme chose cote
interface.

Les resultats sont conserves pour le carnet ouvert : fermer et rouvrir l'app
rend les tableaux, les graphiques et les figures. Changer de carnet reecrit le
precedent sans ses resultats, faute de place dans le megaoctet disponible. Le
noyau, lui, redemarre : le code revient, les variables non, et l'app le dit.

## Donnees

Selecteur de fichiers, glisser-deposer, ou `data.add`. CSV, TSV et JSON. Le
separateur est detecte, les types sont inferes par colonne, et les fichiers
exportes depuis un tableur francais, point-virgule avec virgule decimale, sont
lus correctement. Les jeux de donnees sont partages par tous les carnets.

Sous 24 Kio ils sont sauvegardes ; au-dela ils restent disponibles pour la
session et sont signales comme tels. La jauge de l'onglet Donnees montre
l'occupation reelle.

## Graphiques

Rendus en SVG, sans bibliotheque. Palette categorielle validee avec le
validateur data-viz contre les deux surfaces utilisees ici : pire paire
adjacente en deficience de vision des couleurs dE 8,4 en sombre et 9,1 en
clair, vision normale 19,3 et 19,6. Les teintes sont assignees dans un ordre
fixe et jamais recyclees : au-dela de huit series, la queue est repliee dans une
serie "Autres". `color = "#rrggbb"` remplace la palette quand l'auteur le
demande, en hexadecimal uniquement. Chaque graphique porte son tableau, qui sert
de vue accessible et de relief pour l'avertissement de contraste en theme clair.
Export SVG et PNG, CSV pour les tableaux.

## Interface

Trois formats : panneau lateral des 360 px, fenetre detachee, page pleine. Au
dela de 900 px l'inspecteur devient une colonne. La page elle-meme ne defile
jamais : le carnet et l'inspecteur ont chacun leur ascenseur, ce qui rend le
redimensionnement vertical comme horizontal previsible. Sous 700 px de haut
l'inspecteur cede de la place au carnet.

Le carnet suit l'agent tant que vous etes en bas. Des que vous remontez, le
suivi s'arrete ; il reprend quand vous revenez en bas.

Zones sures respectees, `overscroll-behavior` contenu, cibles tactiles
agrandies sous 520 px, themes clair et sombre suivant l'hote, respect de
`prefers-reduced-motion`.

Francais et anglais. Aucune chaine visible n'est ecrite dans le code : tout
passe par `ui/locales/`, avec pluriels via `Intl.PluralRules` et formatage des
nombres et des dates via `Intl`. Une colonne d'entiers courts n'est pas groupee,
pour qu'une annee se lise 2021 et non 2 021. Ajouter une langue, c'est deposer
un fichier et l'ajouter a `AVAILABLE` dans `lib/i18n.js` ; un test verifie que
les catalogues restent alignes.

## Build et tests

Depuis la racine du depot :

```powershell
node examples/apps/ds-studio/test/engine.test.js      # 55 verifications
node examples/apps/ds-studio/test/notebook.test.js    # 38 verifications
node examples/apps/ds-studio/test/i18n.test.js        # catalogues alignes
node examples/apps/ds-studio/test/browser.test.cjs    # 100 verifications, Chrome
python examples/apps/ds-studio/build.py
```

Le test navigateur sert l'app sur un serveur local avec un faux SDK, puis la
pilote comme le ferait un agent : attente de Pret, lecture du noyau, chargement
de donnees, ajout de cellule, execution, sondage du job, verification que le
tableau et le graphique sont bien dans la page, annulation, revisions perimees,
alias d'arguments, couleur explicite, suivi du defilement, quotas de stockage et
bibliotheque de carnets. Il cherche Chrome ou Edge, ou honore `CHROME_PATH`, et
se declare ignore s'il n'en trouve aucun.

L'archive est ecrite dans `dist/`. Dans LaRuche : **Apps**, **Installer**,
selectionner l'archive, activer l'app. Pour que l'agent puisse s'en servir,
accorder l'ouverture et les actions dans **Permissions**, section
*Agent vers App*. Le panneau Agent de l'app demande en plus `agents.invoke` et
la selection d'un agent dans *App vers agents*.

## Structure

```
package/
  app.json                 manifeste, permissions, 25 actions, guide
  ui/
    index.html
    styles.css
    icon.svg
    locales/fr.json, en.json
    lib/
      frame.js             moteur tabulaire en colonnes
      csv.js               lecture delimitee, inference de types
      lang.js              tokenizer, parser, interpreteur du carnet
      chart.js             rendu SVG
      store.js             registre des jeux de donnees et budgets
      i18n.js              traduction et formatage
    kernel/
      kernel.js            selection
      kernel-js.js         noyau integre et sa reference
      kernel-python.js     adaptateur Pyodide
      ../vendor/pyodide/   vide, rempli par tools/vendor_pyodide.py
    notebook.js            modele, ordonnanceur, instantanes, persistance
    app.js                 pont SDK, interface, bibliotheque, actions
test/
tools/vendor_pyodide.py
build.py
```

`frame.js`, `csv.js`, `lang.js`, `chart.js`, `store.js` et `notebook.js` ne
touchent ni au DOM ni au SDK, ce qui les rend testables sous Node sans
navigateur.

## Limites connues

- Une cellule en cours ne peut pas etre interrompue de force au milieu de son
  calcul : sans worker, le thread principal reste occupe. Le langage integre
  dispose de controles cooperatifs avec un budget d'environ douze secondes;
  ce n'est pas une limite dure pour du Python/WASM. L'annulation avant demarrage marche.
- L'app ne peut pas telecharger un dataset. L'agent le peut et le passe a
  `data.add`, plafonne a 48 000 caracteres par appel par la limite du pont.
- Les resultats ne sont conserves que pour le carnet ouvert, et une figure de
  plus de 96 Kio n'est pas sauvegardee. L'interface indique ce qui a ete ecarte.
- Deux fenetres ouvertes sur le meme carnet ne se synchronisent pas en direct.
  La derniere sauvegarde gagne, comme pour les autres apps.
- Le noyau Python n'a pas ete execute ici : les assets Pyodide ne sont pas dans
  le depot. Le chemin de detection, l'echec propre et le repli sont testes ;
  l'execution Python elle-meme demande de lancer le script de vendoring.

## Correction 1.3.3

- La section 10 interdisait le reseau, le disque et le shell « depuis une
  cellule ». Cette precision ouvrait la porte qu'elle croyait fermer: un agent en
  a deduit que le shell de l'hote, lui, etait libre. Il a lu le stockage prive de
  l'App avec PowerShell, puis fouille les sources du paquet pour deviner comment
  tracer en 3D, alors que le guide documente `scatter3d` et `color`.
- L'interdiction porte desormais sur les outils de l'agent autant que sur les
  cellules, et le cas qui declenche la derive est nomme: un `data.list` vide
  signifie qu'il n'y a pas de donnees, pas qu'elles se cachent sur le disque. La
  reponse est `data.add`.

## Correction 1.3.2

- Le guide listait vingt-cinq actions sans dire une seule fois qu'elles passent
  par `app_call`. Un agent lisait `notebook.state {includeOutputs?}`, l'appelait
  comme un outil de l'hote, recevait `Unknown tool`, puis inventait `revision: 0`
  pour continuer. Les guides du 2048 et des Dames montraient deja l'enveloppe,
  celui-ci l'oubliait.
- La section 3 s'ouvre desormais sur la forme d'appel, avec un exemple complet,
  et rappelle que seuls app_list, app_guide, app_open et app_wait sont des outils
  appelables directement.

## Correction 1.3.1

- Le guide et le noyau annoncaient Python 3.11. Le script de vendoring epingle
  Pyodide 0.26.4, qui embarque CPython 3.12. Ils renvoient desormais au champ
  `version` de `kernel.status` plutot qu'a un numero recopie a la main, puisque
  c'est le runtime vendorise qui decide, et lui seul.
- Le numero de version monte parce que l'installeur refuse une version deja
  presente dans le foyer. Republier le meme 1.3.0 corrige n'aurait rien installe.

## Corrections et extensions 1.3.0

- Chaque modification de dataset invalide la revision, y compris un import ou une
  suppression dans l'interface. Deux noms de dataset de 64 caracteres ne s'ecrasent
  plus lors de l'ajout du suffixe de collision.
- Les morceaux de sauvegarde tiennent compte des octets UTF-8 et de l'echappement
  JSON. Ils sont ecrits un par un, sans depasser les huit appels simultanes du pont.
  Un carnet dont le code seul depasse le budget est refuse avant d'ecrire ses morceaux.
- `storage.status` expose sauvegarde en attente, erreur, derniere revision sauvee,
  datasets non conserves et resultats omis. `storage.flush({revision})` attend une
  sauvegarde et signale ses omissions. Une erreur conserve l'indicateur non sauvegarde.
- `data.export` accepte `offset` et `limit`, rend uniquement des lignes CSV completes
  et donne `nextOffset` pour poursuivre. Les champs quotes, retours a la ligne et
  caracteres Unicode ne sont pas coupes. Chaque page a son propre en-tete.
- Un carnet marque Python reste marque Python si le runtime manque : il n'est pas
  reinterprete sous le langage studio. `kernel.status` et l'etat du carnet exposent
  `compatible`; les executions sont bloquees jusqu'au retour du bon noyau ou la
  creation d'un nouveau carnet adapte.

Le guide demande aussi de verifier unites, monnaies, doublons, valeurs manquantes,
periode partielle et provenance avant une comparaison. Il interdit de presenter le
dataset de demonstration comme les donnees demandees par l'utilisateur.

Le test `examples/apps/test/ds-persistence.test.cjs` utilise le vrai hote et un foyer
temporaire. Il valide sauvegarde, redemarrage, tableaux, graphique et dataset, ainsi
que les nouvelles actions. Voir le README de ce dossier pour les prerequis.

Restent hors de cette version : gros fichiers binaires persistants, sauvegarde
transactionnelle multi-cles resistante a une coupure pendant l'ecriture, export
d'images via actions, Worker avec interruption dure, validation du runtime Pyodide
et execution sans navigateur. Les telechargements de fichiers et d'images sont deja
disponibles manuellement dans l'interface du carnet.
