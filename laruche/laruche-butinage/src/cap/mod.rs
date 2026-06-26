//! `cap` — la **politique** de l'abeille : où elle va, quand elle s'arrête.
//!
//! Tout ce qui décide est ici, isolé de la boucle et **unit-testé** :
//! - [`vigie`] : surveille les boucles stériles (contrôleur pur, sans effet de bord).
//! - [`boussole`] : la seule fonction de continuation (`cap()`).
//! - [`jauge`] : le budget de contexte en tokens réels (à venir avec le moteur).

pub mod boussole;
pub mod jauge;
pub mod vigie;
