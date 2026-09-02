# LaRuche Chrome extension privacy policy

Effective date: September 2, 2026

The LaRuche Chrome extension connects Chrome to a LaRuche node running on the
same computer. It has no advertising, analytics, tracking SDK or developer-run
data collection service.

## Data the extension can access

When browser control is enabled, the extension can access the pages driven by
LaRuche. Depending on the requested browser action, this can include:

- page addresses, titles, visible text and page structure
- open-tab addresses and titles
- clicks, typed values and other actions performed by the agent
- screenshots, console messages and network metadata
- cookie names, domains, flags and sizes, but not cookie values in agent output

The optional **Keep** feature sends the current page title, address and the
comment written by the user to the local LaRuche memory. If the node is not
available, up to 200 pending entries can remain in Chrome local extension
storage until they are sent or the extension is removed.

The optional recording feature can capture a selected tab, Chrome window or
screen after the user enables recording and grants Chrome's permission. The
resulting video is saved through Chrome to the user's Downloads folder or to a
location selected by the user.

The optional video library can access a folder only after the user selects it
through Chrome's folder picker. It reads video names, paths, sizes, dates,
durations, dimensions and frames used to create local thumbnails. When the user
requests an edit, it can rename a file or create a trimmed and cropped export in
that folder. Processing uses Chrome's local media APIs. Videos and thumbnails
are not uploaded.

## How data is used

The extension uses this data only to provide the browser action, memory and
recording features requested by the user. It connects to
`ws://127.0.0.1:<port>`, which is the LaRuche node on the same computer.

The extension itself does not send data to a remote LaRuche service. The local
LaRuche node may send selected conversation or page content to a model, search,
speech or messaging provider configured by the user. That processing is
controlled by the user's LaRuche configuration and the provider's own privacy
terms.

Data accessed by the extension is not sold, used for advertising, used for
credit decisions or transferred for an unrelated purpose.

No developer or employee can read this data because the extension does not send
it to a developer-operated server. Any transfer initiated by the user's local
LaRuche configuration is limited to providing the requested agent feature.

The use of information received from Google APIs will adhere to the Chrome Web
Store User Data Policy, including the Limited Use requirements.

## Local storage and retention

Chrome local extension storage contains settings such as the selected language,
node port, enabled features, recording preferences, companion position and any
pending **Keep** entries. The extension does not set a remote identifier and
does not create an analytics profile.

The video library stores the selected folder handle in extension-owned
IndexedDB. This handle identifies the user-selected local folder and is used
only to reopen the library. Chrome controls the underlying read and write
permission, and the user can revoke it.

Captured videos are retained wherever the user saves them. Entries accepted by
the local node follow LaRuche's local memory settings. Temporary browser-control
results are held only long enough to answer the current local request.

## Permissions and user control

Browser control is disabled by default. The user must enable it in the popup and
can disable it at any time to close the local connection and detach the Chrome
debugger. Recording is also disabled by default and its additional permissions
are requested only when the user enables that feature.

Removing the extension clears its Chrome local storage. LaRuche memory entries
and videos are separate local files and can be deleted by the user from LaRuche
or the file system.

## Security

The browser bridge accepts commands only while the extension's control switch
is enabled. Chrome displays its debugger warning while a tab is under control.
The LaRuche node checks the extension origin and accepts only the configured
extension identifier.

## Changes and contact

Material changes to this policy will be published in this file with a new
effective date. Questions and privacy reports can be submitted through the
[LaRuche GitHub repository](https://github.com/infinition/LaRuche/issues) or as
a private GitHub security advisory when the report contains sensitive details.

## Version francaise

L'extension Chrome LaRuche relie Chrome a un noeud LaRuche execute sur le meme
ordinateur. Elle n'integre ni publicite, ni analytique, ni traceur, ni service
de collecte opere par le developpeur.

Quand le pilotage est active, elle peut lire les adresses, titres, textes et
structures des pages pilotees, consulter les onglets ouverts, effectuer les
actions demandees, prendre des captures et lire les messages de console ou les
metadonnees reseau. Les noms, domaines, attributs et tailles des cookies peuvent
etre consultes, mais leurs valeurs ne sont pas exposees dans la reponse de
l'agent.

La fonction **Garder** envoie au noeud local le titre et l'adresse de la page,
avec le commentaire saisi. Si le noeud est indisponible, jusqu'a 200 entrees
peuvent rester dans le stockage local de l'extension. L'enregistrement est
facultatif, soumis aux autorisations de Chrome et sauvegarde dans le dossier
choisi par l'utilisateur.

La bibliotheque video accede uniquement au dossier choisi explicitement dans le
selecteur de Chrome. Elle lit les noms, chemins, tailles, dates, durees,
dimensions et images necessaires aux vignettes locales. A la demande de
l'utilisateur, elle peut renommer un fichier ou creer dans ce dossier un export
recadre et raccourci. Les videos et les vignettes ne sont pas envoyees sur le
reseau. Le handle du dossier est conserve dans IndexedDB et son autorisation
reste controlee et revocable par Chrome.

L'extension communique uniquement avec le noeud local sur
`ws://127.0.0.1:<port>`. Le noeud LaRuche peut ensuite utiliser un fournisseur
de modele, de recherche, de parole ou de messagerie choisi et configure par
l'utilisateur. L'extension ne vend aucune donnee, ne les utilise pas pour la
publicite et ne les transfere pas pour une finalite sans rapport avec LaRuche.

Le pilotage et l'enregistrement sont desactives par defaut. Couper le pilotage
ferme la connexion locale et detache le debogueur Chrome. Supprimer l'extension
efface son stockage Chrome. Les videos et les entrees deja acceptees par la
memoire LaRuche sont des fichiers locaux distincts que l'utilisateur peut
supprimer lui-meme.
