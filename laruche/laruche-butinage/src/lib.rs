//! # laruche-butinage — le cerveau de l'abeille
//!
//! Moteur ReAct de LaRuche. Une mission est un **butinage** : une quête itérative
//! où l'abeille raisonne, récolte (outils), observe, et recommence jusqu'à ce que
//! l'itinéraire soit accompli.
//!
//! Principe directeur : **la boucle est bête, la politique est isolée et testée.**
//! - [`cycle::butiner`] : la boucle minimale, pilotée par [`issue::Issue`].
//! - [`cap`] : la politique — [`cap::boussole::cap`] décide continuer/poser,
//!   [`cap::vigie`] surveille les boucles stériles, jauge (à venir) le budget.
//! - [`carnet::Carnet`] : l'état persistable (reprise après crash).
//! - [`itineraire::Itineraire`] : le plan = **source de vérité** de la terminaison.
//! - [`meteo`] : classification d'erreurs + politique de retry.
//!
//! Intégration par **inversion de dépendances** : le moteur ne connaît pas les
//! providers/outils concrets, seulement des traits ([`fournisseur::Fournisseur`],
//! [`outils::Outils`], [`evenement::Emetteur`]) que `laruche-essaim` implémente.
//!
//! Aucune heuristique métier (matching de chaînes) dans la boucle : les décisions
//! reposent sur des *faits* (stop_reason natif, itinéraire, compteurs).

pub mod carnet;
pub mod cycle;
pub mod escale;
pub mod evenement;
pub mod fournisseur;
pub mod issue;
pub mod itineraire;
pub mod messagerie;
pub mod meteo;
pub mod outils;
pub mod recolte;
pub mod reglages;
pub mod cap;

pub use carnet::{Carnet, ModeMission};
pub use cycle::butiner;
pub use evenement::{Emetteur, Evenement, Silencieux};
pub use fournisseur::{ErreurFournisseur, Fournisseur, ReponseModele, Usage};
pub use issue::{Appel, Bilan, FinDeVol, Issue, StopReason, TexteSeul};
pub use itineraire::{Etape, Itineraire, StatutEtape};
pub use messagerie::{Message, Role};
pub use meteo::{ClasseErreur, Reaction};
pub use outils::{Outils, ResultatOutil};
pub use reglages::{ProfilModele, Reglages};
pub use cap::boussole::{cap, ContexteCap, Decision};
pub use cap::jauge::{Besoin, Jauge};
pub use cap::vigie::{SeuilsVigie, Signal, Vigie};
