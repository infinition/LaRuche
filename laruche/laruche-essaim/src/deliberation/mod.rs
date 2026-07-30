//! Deliberation multi-agents: un pool de specialistes, une constitution commune, et un
//! debat borne dont le desaccord est le livrable.
//!
//! # Pourquoi ce dispositif
//!
//! Un modele seul repond. Une table ronde produit autre chose: la carte de ce qui est
//! solide et de ce qui ne l'est pas. C'est le desaccord qui vaut le detour - un accord
//! entre modeles, lui, est bon marche et trompeur.
//!
//! # Comment il est construit
//!
//! - [`constitution`] : les regles communes a tous, couche verrouillee.
//! - [`specialiste`] : le pool, qui varie par sa STRATEGIE de raisonnement, pas par sa
//!   personnalite. Livre dans le binaire, surchargeable par l'utilisateur.
//! - [`tour`] : ce qu'un specialiste rend, et le parseur qui alimente les indicateurs.
//! - [`moteur`] : l'enchainement des tours, le plafond de cout, et l'arret.
//!
//! # Les trois pieges qu'il evite
//!
//! **L'effondrement par complaisance.** C'est l'echec connu du debat entre modeles: ils
//! convergent vers la premiere reponse assuree plutot que vers la bonne. Deux gardes:
//! un contradicteur qui n'a pas le droit de se rallier, et l'obligation faite a
//! quiconque change d'avis de dire ce qui l'a fait changer. Une revision devient
//! auditable, une capitulation devient visible.
//!
//! **Le score de consensus.** Un pourcentage d'accord entre modeles mesure la
//! conformite, pas la justesse - et c'est pourtant le chiffre auquel un lecteur se
//! fierait le plus. On rend donc une REPARTITION (qui approuve quoi) et jamais un
//! nombre unique presente comme une confiance.
//!
//! **Le cout.** Chaque participant coute un appel par tour, et les tours de relecture
//! font grossir le contexte de tout ce qui a ete dit. L'orchestrateur ne choisit pas
//! seulement qui parle: il achete le debat, sous plafond.

pub mod constitution;
pub mod executeur;
pub mod moteur;
pub mod outils;
pub mod specialiste;
pub mod tour;

pub use constitution::{constitution_effective, prompt_specialiste, CONSTITUTION, FORMAT_TOUR};
pub use executeur::{deliberer, Appel, Creds, Profils};
pub use moteur::{Arret, Deliberation, Etape, Mission, Plan, Reglages};
pub use outils::{permis, touche_la_machine};
pub use specialiste::{embauches, pool, Role, Specialiste};
pub use tour::{Accord, Intervention};
