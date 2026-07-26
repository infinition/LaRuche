# Evals - la preuve que le moteur marche

Rejoue un jeu de missions FIXE contre le vrai moteur butinage (vrai provider, vrais
outils) et juge chaque run avec des checks durs. À lancer **avant/après tout changement**
de moteur, de prompt ou de modèle : c'est la mesure, pas l'impression.

## Lancer

```bash
# depuis laruche/ (le node peut tourner, l'éval est indépendante)
RUCHE_PROVIDER=ollama RUCHE_MODEL=qwen3:32b cargo run -p laruche-evals
# options
cargo run -p laruche-evals -- --only deep          # sous-ensemble par id
cargo run -p laruche-evals -- --repeat 3           # variance (modèles non déterministes)
cargo run -p laruche-evals -- --judge              # + juge LLM (aux_model)
cargo run -p laruche-evals -- --save-baseline      # fige la référence
```

Env : `RUCHE_PROVIDER`, `RUCHE_MODEL`, `RUCHE_API_KEY`, `RUCHE_API_BASE`, `OLLAMA_URL`,
`RUCHE_CONTEXT_MAX`, `RUCHE_AUX_MODEL`.

## Ce qui est mesuré

- `fin` (accomplie/plafond/erreur/…), `mode` final (standard/exploration - teste les
  canaux de décision : mots-clés, `research_mode`), `passes`, effort web, **fan-out**
  (delegations), tokens, durée ;
- **démission** : l'agent renvoie l'utilisateur chercher ou demande la permission -
  interdit en mission ;
- checks par mission : `min_web`, `min_delegations`, `max_passes`, contenu
  obligatoire/interdit, fichier produit.

## Sorties

- table markdown sur stdout ;
- `evals/results/run-<ts>.jsonl` (diffable machine) ;
- diff vs `evals/baseline.json` : les **RÉGRESSIONS** sont signalées. `--save-baseline`
  pour re-figer après une amélioration validée.

## Étendre

Ajouter une entrée dans `missions.json`. Règles : une mission = un comportement à
protéger (y compris les **contrôles négatifs** : une question triviale ne doit PAS
déclencher l'exploration). Le jeu embarqué est parsé par un test unitaire - une typo
casse le build, pas le run.
