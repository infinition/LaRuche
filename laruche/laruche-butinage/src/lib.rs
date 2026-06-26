//! # laruche-butinage — le cerveau de l'abeille
//!
//! Moteur ReAct de LaRuche. Une mission est un **butinage** : une quête itérative
//! où l'abeille raisonne, récolte (outils), observe, et recommence jusqu'à ce que
//! l'itinéraire soit accompli.
//!
//! Principe directeur : **la boucle est bête, la politique est isolée et testée.**
//! - [`cycle`] : la boucle minimale, pilotée par [`issue::Issue`].
//! - [`cap`] : la politique (où va l'abeille) — [`cap::boussole`] décide continuer/poser,
//!   [`cap::vigie`] surveille les boucles stériles, [`cap::jauge`] le budget contexte.
//! - [`carnet`] : l'état persistable (reprise après crash).
//! - [`itineraire`] : le plan = **source de vérité** de la terminaison.
//!
//! Aucune heuristique métier (matching de chaînes) dans la boucle : les décisions
//! reposent sur des *faits* (stop_reason natif, itinéraire, compteurs).

pub mod carnet;
pub mod issue;
pub mod itineraire;
pub mod cap;

pub use carnet::{Carnet, ModeMission};
pub use issue::{Appel, Bilan, FinDeVol, Issue, StopReason};
pub use itineraire::{Etape, Itineraire, StatutEtape};
pub use cap::boussole::{cap, ContexteCap, Decision};
pub use cap::vigie::{Signal, SeuilsVigie, Vigie};
