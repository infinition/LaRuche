1. **Critique - Le plugin VS Code considère souvent les nœuds “offline” à tort (health parsing)**
- Le client VS Code parse **toutes** les réponses en JSON: [client.ts:100](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-vscode/src/client.ts:100)
- `health()` appelle `/health` via ce parseur JSON: [client.ts:179](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-vscode/src/client.ts:179)
- Or `/health` renvoie du texte `"OK"` côté node/dashboard: [laruche-node/main.rs:785](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-node/src/main.rs:785), [laruche-dashboard/main.rs:17](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-dashboard/src/main.rs:17)
- Impact: faux négatifs santé, auto-connect/local-probe dégradés.

2. **Élevée - Divergence entre `LaRuche` et ton repo local `land-protocol`**
- `LaRuche` dépend de `land-protocol` Git distant: [LaRuche/Cargo.toml:18](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/Cargo.toml:18)
- Lock actuel sur commit `49e4891...`: [Cargo.lock:956](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/Cargo.lock:956)
- Ce commit cache a `NODE_STALE_TIMEOUT_SECS = 15`: [cargo-checkout/discovery.rs:16](/C:/Users/infinition/.cargo/git/checkouts/land-protocol-69879b049e5fc925/49e4891/src/discovery.rs:16)
- Ton repo local a `30`: [local land-protocol/discovery.rs:16](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/discovery.rs:16)
- Impact: comportements différents selon ce que tu testes vs ce qui est réellement utilisé par `LaRuche`.

3. **Élevée - Doctests cassés dans `laruche-client` (et `cargo test --workspace` échoue)**
- Import doc invalide (`Capability` non exporté sous ce nom): [lib.rs:22](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-client/src/lib.rs:22)
- Exemple doc appelle une méthode inexistante (`transcribe`): [lib.rs:33](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-client/src/lib.rs:33)
- Exemple doc `ask` sans contexte async correct: [lib.rs:185](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-client/src/lib.rs:185)
- Alias exporté réel: [lib.rs:96](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-client/src/lib.rs:96)

4. **Élevée - QoS partiellement non fonctionnel dans `land-protocol`**
- Les compteurs `active_*` pilotent `should_degrade` / `accepting_qos`: [qos.rs:115](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/qos.rs:115), [qos.rs:165](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/qos.rs:165), [qos.rs:180](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/qos.rs:180)
- Mais ils ne sont jamais incrémentés à la prise en charge (`dequeue` pop seulement): [qos.rs:146](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/qos.rs:146)
- Ils sont seulement décrémentés via `complete`: [qos.rs:153](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/qos.rs:153)
- Impact: la logique de saturation/priorité est incohérente.

5. **Élevée - `load_config()` annonce un fichier chargé mais ne le parse pas**
- Message “Loaded config from file” si le fichier existe: [laruche-node/main.rs:933](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-node/src/main.rs:933)
- Ensuite config construite uniquement via env vars: [laruche-node/main.rs:958](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-node/src/main.rs:958)
- Impact: confusion ops, faux sentiment que `laruche.toml` est pris en compte.

6. **Élevée - Auth PoP non appliquée sur `/infer`**
- InferenceRequest n’a pas de token/auth field: [laruche-node/main.rs:101](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-node/src/main.rs:101)
- `/infer` traite sans validation auth: [laruche-node/main.rs:552](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-node/src/main.rs:552)
- Auth existe mais reste séparée (`/auth/request`, `/auth/approve`): [laruche-node/main.rs:870](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-node/src/main.rs:870)

7. **Moyenne - Fragilité IPv6 (LAND/clients)**
- `land-protocol` peut prendre une adresse non normalisée depuis mDNS: [discovery.rs:192](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/discovery.rs:192), [discovery.rs:194](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/discovery.rs:194)
- URLs construites en `http://{host}:{port}` sans bracket IPv6: [laruche-node/main.rs:287](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-node/src/main.rs:287), [manifest.rs:360](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/manifest.rs:360)
- VS Code discovery ignore les services sans IPv4: [discovery.ts:227](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-vscode/src/discovery.ts:227)
- Impact: support IPv6 incomplet/incohérent.

8. **Moyenne - Schéma `/swarm` perd le port peer pour les clients**
- `DiscoveredNodeInfo` expose `host` mais pas `port`: [laruche-node/main.rs:177](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-node/src/main.rs:177)
- Le plugin reconstruit en supposant `8419`: [extension.ts:231](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-vscode/src/extension.ts:231), [extension.ts:397](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-vscode/src/extension.ts:397)
- Impact: si un node écoute sur un port custom, découverte OK mais ciblage client potentiellement faux.

9. **Moyenne - Cohérence état swarm dans `land-protocol::swarm`**
- `add_peer` met `Syncing`: [swarm.rs:139](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/swarm.rs:139)
- `heartbeat` ne promeut que `Suspect -> Active`: [swarm.rs:162](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/swarm.rs:162)
- `active_peer_count`/`plan_sharding` ne prennent que Active/Busy: [swarm.rs:193](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/swarm.rs:193), [swarm.rs:226](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/swarm.rs:226)
- Impact: des peers ajoutés peuvent rester exclus du sharding sans transition explicite.

10. **Moyenne - Bug UI dashboard: classe CSS `fill-blue` non définie**
- Utilisée pour la queue bar: [dashboard.html:365](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-dashboard/src/templates/dashboard.html:365)
- Mais seules `fill-green/amber/red` existent: [dashboard.html:184](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-dashboard/src/templates/dashboard.html:184)
- Impact: barre queue potentiellement sans couleur attendue.

11. **Moyenne - Doc protocole en décalage avec l’implémentation**
- README dit “Gzipped JSON payload” en mDNS: [land-protocol/README.md:32](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/README.md:32)
- Implémentation actuelle repose sur TXT props: [manifest.rs:230](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/manifest.rs:230), [manifest.rs:271](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/land-protocol/src/manifest.rs:271)
- Impact: intégrateurs tiers peuvent implémenter le mauvais format.

12. **Faible - `laruche-client::connect()` parsing URL fragile**
- Parse host/port par `split(':')` et `replace("http://")`: [lib.rs:146](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-client/src/lib.rs:146), [lib.rs:159](/C:/DEV/Workspace/active/coding/_AI%20RESEARCH/LaRuche/laruche-client/src/lib.rs:159)
- Impact: fragile avec IPv6, URLs non standards, chemins/query.

**Résultats de vérification exécutés**
- `cargo check --workspace` dans `LaRuche`: OK
- `cargo test --workspace` dans `LaRuche`: échec sur doctests `laruche-client`
- `cargo check` dans `land-protocol`: OK
- `cargo test` dans `land-protocol`: non exécutable ici (environnement/toolchain `dlltool.exe` manquant)
- `npm run compile` dans `laruche-vscode`: OK

**Remarque synthèse**
- Le socle compile, mais il y a des écarts importants entre intention (LAND/QoS/Auth/docs) et comportement effectif (health client, QoS runtime, config file, dépendance protocole).  
- Le point le plus urgent à corriger en production est le `health` du plugin VS Code.