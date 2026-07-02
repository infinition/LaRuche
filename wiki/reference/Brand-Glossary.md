# Brand Glossary

The hive speaks French. The brand vocabulary is part of LaRuche's identity and stays
French in the code, the UI, and the docs; everything else is English. Here is the
decoder ring.

| Term | Literal meaning | In LaRuche |
|---|---|---|
| **LaRuche** | the hive | The project and the node itself |
| **butinage** | bees foraging | The agentic ReAct loop, the engine |
| **butiner** | to forage | To run an agentic mission |
| **essaim** | swarm | The agent layer (crate `laruche-essaim`) |
| **abeille** | bee | A tool the agent can call |
| **éclaireuse** | scout bee | A parallel sub-agent for research fan-out |
| **escale** | stopover | Context compaction mid-run |
| **boussole** | compass | The engine's planning state |
| **jauge** | gauge | The token budget accountant |
| **vigie** | lookout | The anti-loop sentinel, also the watchers UI |
| **curateur** | curator | The background component that grows the skill library |
| **LaReine** | the queen | The built-in supervisor and judge |
| **Miel** | honey | The mesh protocol between nodes |
| **nectar** / **Source** | nectar / source | Memory content and its store |

Two naming rules for contributors: identifiers that carry brand vocabulary keep it
(`butiner()`, `AbeilleRegistry`), and no em dashes anywhere in the codebase or docs.
