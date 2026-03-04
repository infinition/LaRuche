# LaRuche - Roadmap & Spécifications Futures

Ce document consolide les fonctionnalités prévues (issues du README) et détaille les suggestions d'implémentation ainsi que les défis techniques associés pour le projet LaRuche.

## Fonctionnalités implémentées (Terminées)

Ces éléments sont déjà opérationnels dans la version actuelle du projet :

- [x] **LAND protocol core** (mDNS discovery + Cognitive Manifest)
- [x] **Capability differentiation** (LLM, VLM, VLA, RAG, Audio, Image, Embed, Code)
- [x] **Proof of Proximity authentication**
- [x] **QoS priority system**
- [x] **Swarm state management**
- [x] **Node daemon with Ollama bridge**
- [x] **Client SDK** (3-line usage)
- [x] **CLI tool**
- [x] **Web dashboard with cyber monitoring**

## Fonctionnalités prévues (Backlog)

Ces éléments ne sont pas encore implémentés et font partie de la roadmap officielle du projet :

### 🔬 Preuves de Concept (POC) & Intégrations Capacitaires
Ces points visent à tester et valider concrètement les différents modèles et cas d'usage pris en charge par le système de capacités (`capability:*`) du protocole LAND :
- [ ] **POC `capability:llm` (Text-to-text)** : Le cas d'usage de base. Un nœud standard exécutant un modèle comme Mistral ou Llama pour de la discussion fluide ou des tâches NLP simples.
- [ ] **POC `capability:vlm` (Vision-Language)** : Envoyer une image via le réseau à un nœud exécutant LLaVA ou Qwen-VL pour analyse visuelle, description ou OCR.
- [ ] **POC `capability:vla` (Vision-Language-Action)** : Piloter un bras robotique ou un drone basique via le réseau LaRuche.
- [ ] **POC `capability:rag` (Retrieval-Augmented Generation)** : Créer un nœud LaRuche spécialisé qui indexe un dossier de documents locaux et répond aux questions sur cette base.
- [ ] **POC `capability:audio` (Speech-to-Text / Text-to-Speech)** : Intégrer un nœud Whisper/Bark pour permettre des commandes vocales au réseau.
- [ ] **POC `capability:image` & `capability:embed`** : Générer des images (ex: Stable Diffusion) ou des vecteurs depuis un nœud dédié à la volée.
- [ ] **POC `capability:code`** : Tester un nœud exécutant CodeLlama/DeepSeek-Coder pour des requêtes spécifiques de développement.

### 🏗️ Infrastructure & Écosystème
- [ ] **Tensor sharding over Ethernet (Swarm Intelligence)**
- [ ] **LaRuche Resilience (failover, hot-swap, mirroring)**
- [ ] **NFC hardware integration**
- [ ] **VS Code extension**
- [ ] **Home Assistant plugin**
- [ ] **Mobile app (iOS/Android)**
- [ ] **LAND v1.0 specification (RFC)**

---

## Pistes de développement et Défis Techniques

Voici une déclinaison plus détaillée des points de la roadmap pour guider l'implémentation.

### 1. Swarm Intelligence & Tensor Sharding
Le "Tensor sharding over Ethernet" est l'un des aspects les plus novateurs et complexes du projet, nécessitant de surmonter les limites de réseau local.
*   **POC de partitionnement :** Réaliser une preuve de concept permettant de diviser l'inférence d'un LLM sur plusieurs machines connectées (pipeline parallelism).
*   **Optimisation réseau :** Explorer des protocoles bas-niveau (UDP, direct TCP) pour minimiser la latence inter-nœuds, potentiellement avec du RDMA si le matériel le permet.

### 2. LaRuche Resilience (Tolérance aux pannes)
Pour que le « plug-and-play de l'IA » soit fiable, la perte d'un boîtier ne doit pas impacter l'utilisateur.
*   **Failover dynamique :** Si un client requiert un nœud qui devient injoignable en cours de route, la requête doit être redirigée de manière transparente vers un autre nœud capable.
*   **Mirroring de contexte :** Synchroniser l'état (ou l'historique de conversation) entre les nœuds d'un même cluster LaRuche pour une reprise sans coupure.

### 3. Écosystèmes Applicatifs (VS Code, Home Assistant, Mobile)
*   **VS Code Extension :** S'appuyer sur la `capability:code`. L'extension découvrira automatiquement le nœud LaRuche via le protocole LAND pour offrir de l'autocomplétion (Copilot local) ou du chat.
*   **Home Assistant Plugin :** Transformer le réseau LaRuche en "cerveau" de la maison. Il faudra intégrer des mécanismes de *Tool Use* (appels de fonctions) pour que le LLM puisse exécuter des actions concrètes (allumer les lumières, etc.).
*   **Mobile App :** Un client léger (iOS/Android) communiquant via le SDK, avec intégration vocale potentielle (Speech-to-Text en amont).

### 4. Hardware et Sécurité (NFC & Proof of Proximity)
La promesse du "zéro configuration" s'accorde parfaitement avec le matériel.
*   **NFC "Tap-to-Connect" :** Approcher un smartphone ou une carte NFC du boîtier LaRuche pour autoriser instantanément un appareil, combinant sécurité ("Proof of Proximity") et expérience utilisateur magique.
*   **Quotas et Pare-feu logique :** Éviter qu'un seul client accapare toute la puissance GPU du réseau en exploitant les indicateurs de QoS du *Cognitive Manifest*.

### 5. Standards et Protocole (LAND v1.0 RFC)
*   **Spécification formelle :** Rédiger la RFC du protocole LAND en détaillant la structure du manifeste mDNS (types TXT), les flags de capacité, et les protocoles d'échange, afin de permettre à d'autres projets d'adopter LAND.
