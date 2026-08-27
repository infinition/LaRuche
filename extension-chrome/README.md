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
4. Cliquer sur l'icone de la ruche, verifier le port du noeud (`8419` par
   defaut, voir `LARUCHE_PORT`), puis activer **Autoriser le pilotage**.

La pastille passe a l'ambre quand la connexion au noeud est etablie.

## Ce que vous voyez pendant le pilotage

- Les onglets de l'agent sont regroupes dans un groupe **LaRuche** jaune.
- La page pilotee porte un cadre ambre qui respire et un badge en bas a droite.
- Chaque element clique ou rempli s'illumine une demi-seconde.
- Chrome affiche sa propre banniere de debogage, que l'extension ne masque pas.

Couper l'interrupteur du popup reprend la main immediatement : la connexion se
ferme, l'agent perd le navigateur et recoit une erreur explicite.

## Ce que l'extension sait faire

Trois primitives seulement : naviguer, evaluer du JavaScript, capturer l'ecran.
Toute la logique utile (cartographier une page, cliquer un element par son
numero, dessiner l'indicateur) est du JavaScript construit par LaRuche et envoye
sur le canal. Une seule implementation sert donc les deux transports, et cette
extension n'a pas a etre mise a jour quand l'outil gagne une capacite.

Le JavaScript est execute via `chrome.debugger` et non `chrome.scripting`, parce
qu'executer du texte de script recu a l'execution passe forcement par `eval` ou
`new Function`, tous deux soumis a la CSP du site visite. Sur la plupart des
sites interessants, l'injection echouerait.

## Securite

- La connexion ne sort jamais de la machine : `127.0.0.1`, port du noeud local.
- Le noeud refuse toute connexion dont l'origine n'est pas une extension. Les
  websockets echappant a la politique CORS, une page web quelconque pourrait
  sinon se connecter et repondre a la place de l'extension.
- Aucune donnee n'est envoyee ailleurs, aucune analytique, aucun serveur tiers.
- La permission `debugger` est large par nature. Elle n'est utilisee que sur
  l'onglet du groupe LaRuche, jamais sur vos autres onglets.

## Protocole

```
noeud     -> extension   {"id":1,"action":"eval","params":{"script":"..."}}
extension -> noeud       {"id":1,"ok":true,"result":{"value":...}}
                         {"id":1,"ok":false,"error":"..."}
```

Actions reconnues : `navigate`, `eval`, `screenshot`, `glow`, `tab`, `close`,
`ping`. Cote LaRuche, le point de rendez-vous est
`laruche-essaim/src/pont_navigateur.rs` et la route est `/ws/navigateur`.
