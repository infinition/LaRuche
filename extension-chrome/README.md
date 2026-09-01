# Extension LaRuche pour Chrome

Laisse l'agent piloter **votre** navigateur, avec vos sessions deja ouvertes,
plutot qu'un Chrome vierge lance a cote.

## Pourquoi une extension

Depuis Chrome 136, `--remote-debugging-port` est ignore sur le profil par
defaut. Un programme exterieur ne peut donc plus se brancher sur le navigateur
que vous utilisez, ce qui est precisement le but de la restriction. Une
extension est la seule voie restante pour agir dans une session connectee.

L'outil `browser` fonctionne sans elle, en mode `launch` : LaRuche demarre alors
son propre Chrome sur un profil separe mais persistant, ou vous pouvez vous
connecter une fois pour toutes. L'extension est un confort, pas une dependance.

## Installation

1. Ouvrir `chrome://extensions`.
2. Activer le **mode developpeur** en haut a droite.
3. **Charger l'extension non empaquetee**, puis choisir ce dossier.
4. Verifier que l'identifiant affiche est bien
   `ahgfjacmpohglimmcfnlbeccdghpkboo`. Le champ `key` du manifeste le fige, donc
   il ne depend pas de l'endroit ou se trouve le dossier.
5. Cliquer sur l'icone de la ruche, verifier le port du noeud (`8419` par
   defaut, voir `LARUCHE_PORT`), puis activer **Autoriser le pilotage**.

La pastille passe a l'ambre quand la connexion au noeud est etablie.

Le popup s'ouvre en anglais sur une nouvelle installation. Le bouton **FR**
bascule l'interface en francais et le bouton **EN** la remet en anglais. Le
choix est conserve localement pour les ouvertures suivantes.

Si le noeud refuse la connexion en journalisant un identifiant different, c'est
que le manifeste a ete modifie ou qu'il s'agit d'un autre build. Pointer
`LARUCHE_EXTENSION_ID` sur le nouvel identifiant, ou restaurer le `key`.

La cle privee de signature ne doit pas se trouver dans le dossier charge par
Chrome. Elle est conservee localement sous `.private/`, qui est ignore par Git.
Le champ public `key` du manifeste suffit pour garder un identifiant stable en
developpement.

## Ce que vous voyez pendant le pilotage

- Les onglets de l'agent sont regroupes dans un groupe **LaRuche** jaune.
- Chrome active automatiquement l'onglet que l'agent ouvre, selectionne ou
  navigue, y compris s'il se trouve dans une autre fenetre.
- La page pilotee porte un cadre ambre qui respire et un badge en bas a droite.
- L'option **Curseur abeille de l'agent** remplace la fleche virtuelle par
  l'abeille. Elle suit exactement les deplacements et les clics du moteur.
- Chaque element clique ou rempli s'illumine une demi-seconde.
- Chrome affiche sa propre banniere de debogage, que l'extension ne masque pas.

Le curseur abeille est desactive par defaut et reste independant du compagnon
abeille. Le compagnon vit librement sur les pages. Le curseur abeille apparait
uniquement dans le halo de pilotage et disparait avec lui.

Le panneau LaRuche conserve sa position, sa taille, son historique, la narration
et un brouillon non envoye quand la page navigue ou est rechargee avec F5. Le DOM
de la page est bien recree par Chrome, mais l'extension restaure le panneau au
demarrage du document suivant pour que l'exploration reste continue.
Il reste visible pendant vingt-quatre secondes apres la derniere activite de
l'agent, sauf si vous le survolez ou si une reponse est en cours de saisie.

## Enregistrer un showcase

Le popup peut enregistrer l'onglet actif, une fenetre choisie ou un ecran
complet. Activez l'enregistrement, choisissez la source, puis cliquez sur
**Demarrer le showcase**. La capture continue dans un document hors ecran, y
compris pendant les navigations et apres la fermeture du popup.

La video est arretee et sauvegardee automatiquement lorsque LaRuche ferme la
session de navigation ou apres quarante secondes sans commande. Le bouton
**Arreter et sauvegarder** permet aussi de terminer manuellement.

Chrome 126 et les versions suivantes produisent un fichier MP4. Un ancien Chrome
qui ne propose pas l'encodeur MP4 utilise WebM comme solution de repli. Le
sous-dossier configure est relatif au dossier Telechargements de Chrome. Activez
**Demander l'emplacement a chaque fichier** pour choisir un autre chemin lors de
chaque sauvegarde.

La capture de l'onglet commence apres un clic explicite sur le popup, comme
l'exige `chrome.tabCapture`. Pour une fenetre ou un ecran complet, Chrome affiche
toujours son propre selecteur de partage. Une extension ne peut pas contourner ce
choix de confidentialite.

Couper l'interrupteur du popup reprend la main immediatement : la connexion se
ferme, l'agent perd le navigateur et recoit une erreur explicite.

Sans commande pendant quarante secondes, l'extension considere le pilotage termine :
elle detache le debogueur, la banniere disparait, et un onglet emprunte retourne
dans le groupe d'ou il venait.

## Ce que l'extension sait faire

Le vocabulaire est volontairement etroit. Toute la logique utile (cartographier
une page, cliquer un element par son numero, dessiner l'indicateur, enregistrer
la console) est du JavaScript construit par LaRuche et envoye sur le canal. Une
seule implementation sert donc les deux transports, et cette extension n'a pas a
etre mise a jour quand l'outil gagne une capacite.

| Action | Ce qu'elle fait |
|---|---|
| `navigate` | Charge une URL et attend `readyState complete` |
| `eval` | Evalue un script dans la page et **renvoie sa valeur** |
| `screenshot` | Capture PNG en base64 |
| `glow` | Installe ou retire l'indicateur de pilotage, reinjecte a chaque navigation |
| `tap` | Installe l'enregistreur console et reseau, une fois par onglet attache |
| `cdp` | Passthrough DevTools brut |
| `tab` | Cree ou retrouve l'onglet pilote, le met au premier plan |
| `tabs` | Liste tous les onglets de toutes les fenetres, en lecture seule |
| `open_tab` | Ouvre et pilote un nouvel onglet dans la meme fenetre Chrome |
| `select` | Adopte un onglet existant comme onglet pilote |
| `close` | Rend l'onglet emprunte, detache le debogueur |
| `ping` | Maintien de la connexion, ne compte pas comme une commande |

Quand le noeud local est arrete, l'extension sonde d'abord `/health` et attend
silencieusement sa disponibilite avant d'ouvrir la WebSocket. L'etat
**Connexion au noeud...** est donc normal pendant le demarrage de LaRuche et ne
signifie pas que l'extension est cassee.

`eval` renvoie bien la valeur produite : `Runtime.evaluate` est appele avec
`returnByValue` et `awaitPromise`, et c'est ce retour qui porte `read`, `find`
et le clic par ref. Une valeur non serialisable en JSON (noeud DOM, fonction,
objet circulaire) revient nulle, ici comme en CDP direct.

Le JavaScript est execute via `chrome.debugger` et non `chrome.scripting`, parce
qu'executer du texte de script recu a l'execution passe forcement par `eval` ou
`new Function`, tous deux soumis a la CSP du site visite. Sur la plupart des
sites interessants, l'injection echouerait.

`cdp` existe pour la seule chose qu'`eval` ne peut pas faire : un evenement
`Input.*`. Une frappe synthetisee depuis un script de page porte
`isTrusted: false`, que les controles natifs et beaucoup de sites ignorent. Cote
extension, ce passthrough n'est pas restreint au domaine `Input` : le noeud peut
y envoyer n'importe quelle methode du protocole. C'est assume, et c'est la raison
pour laquelle l'identite du noeud compte autant que celle de l'extension.

## Securite

- La connexion ne sort jamais de la machine : `127.0.0.1`, port du noeud local.
- Le noeud n'accepte que **cette** extension, comparee sur son identifiant. Les
  websockets echappant a la politique CORS, une page web quelconque pourrait
  sinon se connecter et repondre a la place de l'extension ; et une autre
  extension installee pourrait prendre le canal, puisque le pont remplace sans
  bruit celle qui etait connectee.
- Aucune donnee n'est envoyee ailleurs, aucune analytique, aucun serveur tiers.
- La permission `debugger` est large par nature. Elle n'est utilisee que sur
  l'onglet du groupe LaRuche, jamais sur vos autres onglets.
- Ce qui reste ouvert : l'extension se connecte a ce qui ecoute sur le port
  configure, sans verifier que c'est bien LaRuche. Noeud eteint, un autre
  processus local qui prend le port obtient du CDP brut sur votre Chrome. Ne
  laissez pas le pilotage actif quand le noeud ne tourne pas.

## Protocole

```
noeud     -> extension   {"id":1,"action":"eval","params":{"script":"..."}}
extension -> noeud       {"id":1,"ok":true,"result":{"value":...}}
                         {"id":1,"ok":false,"error":"..."}
```

Cote LaRuche, le point de rendez-vous est
`laruche-essaim/src/pont_navigateur.rs`, la route est `/ws/navigateur`
(`laruche-node/src/ws_navigateur.rs`), et les scripts envoyes sur le canal
vivent dans `laruche-essaim/src/abeilles/navigateur.rs`.
