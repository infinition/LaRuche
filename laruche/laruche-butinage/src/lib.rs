//! # laruche-butinage: the bee's brain
//!
//! LaRuche's ReAct engine. A mission is a **butinage**: an iterative quest
//! where the tool reasons, harvests (tools), observes, and repeats until
//! the itinerary is accomplished.
//!
//! Guiding principle: **the loop is dumb, the policy is isolated and tested.**
//! - [`cycle::butiner`]: the minimal loop, driven by [`issue::Issue`].
//! - [`cap`]: the policy: [`cap::boussole::cap`] decides continue/land,
//!   [`cap::vigie`] watches for sterile loops, jauge (upcoming) the budget.
//! - [`carnet::Carnet`]: the persistable state (recovery after crash).
//! - [`itineraire::Itineraire`]: the plan = **source of truth** for termination.
//! - [`meteo`]: error classification + retry policy.
//!
//! Integration via **dependency inversion**: the engine does not know the
//! concrete providers/tools, only traits ([`fournisseur::Fournisseur`],
//! [`outils::Outils`], [`evenement::Emetteur`]) that `laruche-essaim` implements.
//!
//! No business heuristics (string matching) in the loop: decisions
//! rest on *facts* (native stop_reason, itinerary, counters).

pub mod carnet;
pub mod cycle;
pub mod eclaireuse;
pub mod escale;
pub mod evenement;
pub mod fournisseur;
pub mod issue;
pub mod itineraire;
pub mod messagerie;
pub mod meteo;
pub mod nectar;
pub mod outils;
pub mod recolte;
pub mod reglages;
pub mod cap;

pub use carnet::{Carnet, ModeMission};
pub use cycle::butiner;
pub use eclaireuse::{depecher, OrdreEclaireuse, Rapport, Role as RoleEclaireuse};
pub use evenement::{Emetteur, Evenement, Silencieux};
pub use fournisseur::{ErreurFournisseur, Fournisseur, ReponseModele, Usage};
pub use issue::{Appel, Bilan, FinDeVol, Issue, StopReason, TexteSeul};
pub use itineraire::{Etape, Itineraire, StatutEtape};
pub use messagerie::{Message, Piece, Role};
pub use meteo::{ClasseErreur, Reaction};
pub use nectar::Source;
pub use outils::{Outils, ResultatOutil};
pub use recolte::{Moisson, plafonner_observation};
pub use reglages::{ProfilModele, Reglages};
pub use cap::boussole::{cap, ContexteCap, Decision};
pub use cap::jauge::{Besoin, Jauge};
pub use cap::vigie::{SeuilsVigie, Signal, Vigie};
