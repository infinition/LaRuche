---
type: skill
name: laruche
description: What LaRuche is: architecture, butinage, memory, LaReine, and its full wiki.
tools: [file_read]
---

# LaRuche, expliquee par elle-meme

Ce skill embarque le wiki complet de LaRuche, 28 pages, 126 Ko de markdown. C'est
la MEME source que le site publie: `wiki/` a la racine du depot, dont `docs/wiki.html`
et ce dossier sont deux sorties generees par `scripts/build_wiki.py`. Ce qui est ecrit
ici fait donc foi. Ne repondez jamais de memoire sur le fonctionnement de LaRuche:
ouvrez la page et citez-la.

## Quand se servir de ce skill

Toute question sur LaRuche elle-meme: ce qu'elle sait faire, comment elle est faite, ce
qu'un mot du vocabulaire designe, pourquoi elle se comporte d'une certaine facon, ce
qu'elle fait de vos donnees. Y compris la demande d'accueil "Presente-toi".

Ce skill dit ce que LaRuche EST. Pour FAIRE quelque chose, les skills voisins sont plus
directs: `configure-laruche` (reglages, fournisseur, canaux, secrets),
`cognitive-memory` (retenir et retrouver), `extend-toolset` (plugin, MCP),
`delegation` (sous-agent), `long-running-work` (mission, kanban, plan).

## En une phrase

LaRuche est un agent local: le butinage est sa boucle de raisonnement et d'action, les
abeilles sont ses outils, les eclaireuses ses sous-agents, LaReine sa supervision, et sa
memoire est une carte de faits au format OKF, lisible et modifiable a la main.

## La carte des pages

Ouvrez la page qui repond, avec `file_read`, sur le chemin donne relatif au dossier de
ce skill (`skills/laruche/`). Les corps sont integraux: ils ne coutent rien tant que
vous ne les demandez pas.

| Page | Titre | Ce qu'elle couvre |
|---|---|---|
| **Overview** | | |
| `wiki/Home.md` | LaRuche Wiki | Welcome to the hive. LaRuche is a local-first AI agent with a desktop application, a server node and a lightweight LAN client. |
| `wiki/FAQ.md` | FAQ | No. With llama.cpp or Ollama, a local embedding model and local speech backends, the engine, memory, automation and voice run... |
| `wiki/Security.md` | Security | LaRuche's security model starts from an honest premise: the model will eventually do something wrong. |
| **Getting started** | | |
| `wiki/getting-started/Installation.md` | Installation | The normal entry point is the desktop application. |
| `wiki/getting-started/Desktop-App.md` | Desktop App | LaRuche ships as a desktop application, a server node and a terminal client. |
| `wiki/getting-started/Quick-Start.md` | Quick Start | You have opened the desktop application or started the node ([Installation](Installation)). |
| `wiki/getting-started/Local-Models.md` | Local Models | LaRuche is designed local-first, and specifically for the reality of local models: they are slower, they misformat tool calls,... |
| **Concepts** | | |
| `wiki/concepts/Architecture.md` | Architecture | LaRuche separates the desktop shell, the server node and the terminal client. |
| `wiki/concepts/Butinage-Engine.md` | The Butinage Engine | Butinage is LaRuche's agent loop. It turns a request into a sequence of model calls, tool calls, observations and decisions,... |
| `wiki/concepts/Cognitive-Memory.md` | Cognitive Memory | LaRuche's memory is not a vector store bolted onto a chat log. |
| `wiki/concepts/LaReine.md` | LaReine | LaReine is LaRuche's supervision layer. It can review a finished answer, send the worker through a fresh agentic run, gate... |
| `wiki/concepts/Table-Ronde.md` | Table Ronde | The table ronde is LaRuche's structured multi-agent deliberation mode. |
| `wiki/concepts/Watchers.md` | Watchers | A watcher is a standing condition. It observes something, decides whether that means anything, and reacts. |
| `wiki/concepts/Automation.md` | Automation | The automation hub gathers everything the hive does on its own: crons, missions, the kanban, and [watchers](Watchers). |
| `wiki/concepts/Skills-and-Curator.md` | Skills and the Curator | A skill is a markdown file: instructions, examples, and conventions for a category of task. |
| **Guides** | | |
| `wiki/guides/Computer-and-Browser.md` | Computer and Browser | LaRuche can act on the desktop, the user's Chrome session and visual inputs. |
| `wiki/guides/Chrome-Extension.md` | Chrome Extension | The LaRuche extension lets `browser` control the Chrome instance the user already has open, including its tabs and signed-in... |
| `wiki/guides/Training-Datasets.md` | Training Datasets | LaRuche can turn completed LaReine reviews into training data. |
| `wiki/guides/Voice.md` | Voice | LaRuche can speak and listen through local or remote speech backends. |
| `wiki/guides/Telegram.md` | Telegram | Telegram is LaRuche's most mature remote channel: full agent runs from your phone, persistent per-channel memory, voice... |
| `wiki/guides/MCP.md` | MCP | LaRuche sits on both sides of the Model Context Protocol: it consumes external MCP servers as extra tools, and it exposes... |
| `wiki/guides/Secrets.md` | Secrets | The secrets vault exists for one reason: your API keys, tokens, and passwords should never enter a model's context. |
| `wiki/guides/Troubleshooting.md` | When something does not work | Symptoms first. Each one below has been seen, and the cause is rarely where the message points. |
| **Reference** | | |
| `wiki/reference/Configuration.md` | Configuration | Two layers: supported environment variables set at launch, and the Settings UI for everything that can change live. |
| `wiki/reference/Providers-and-Profiles.md` | Providers and profiles | Most confusion about "which model am I actually talking to" comes from one idea that is never stated: **the active model is a... |
| `wiki/reference/Tools.md` | Tools | LaRuche registers 89 built-in tools in a default node build. |
| `wiki/reference/Evals.md` | Knowing whether an engine change helped | Prompts and loops are the easiest part of an agent to change and the hardest part to judge. |
| `wiki/reference/Brand-Glossary.md` | Brand Glossary | The hive speaks French. The brand vocabulary is part of LaRuche's identity and stays French in the code, the UI, and the docs;... |

## Deux reflexes

Citez la page d'ou vient votre reponse, l'utilisateur peut la relire.
Une page absente de ce tableau n'existe pas: dites-le plutot que de l'inventer.
