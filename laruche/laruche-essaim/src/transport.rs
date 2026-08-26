//! Where LaRuche chooses HOW to speak HTTP, as opposed to what to ask for.
//!
//! Most walls an agent meets are cheap: a User-Agent filter, a missing header.
//! `reqwest` clears those, and it is what every fetch uses. A smaller set of
//! hosts fingerprints the TLS handshake itself (JA3/JA4) and refuses anything
//! whose ClientHello is not a real browser's. No header will fix that, because
//! the rejection happens before the first byte of HTTP is sent.
//!
//! Answering it needs a client built on a browser's own TLS stack, which means
//! BoringSSL, which means cmake, nasm and a C toolchain on every machine that
//! builds LaRuche. That is a heavy price for a capability few targets need, and
//! a bad trade for a project that has to build reliably on a demo laptop.
//!
//! So the emulating client lives behind the `tls-emulation` feature, OFF by
//! default. The default build keeps exactly the dependency graph it has today,
//! and this module is the single seam where the choice is made. Turning the
//! feature on changes which client walled hosts get; it changes nothing else.
//!
//! The seam also decides WHEN, and that is where [`crate::memoire_hotes`] pays
//! off a second time: we already know which hosts wall the direct route, so the
//! expensive client is reserved for them instead of being used everywhere.
//!
//! STATUS: the `tls-emulation` implementation is written but was NOT compiled
//! here. The machine has cmake and nasm but no working BoringSSL toolchain, so
//! the feature is unverified and must be built once before being relied on.
//! Nothing in the default path depends on it.

/// Which client should carry a request to this URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pile {
    /// `reqwest` with a browser User-Agent. Clears header-level filters, which
    /// is the overwhelming majority of what an agent runs into.
    Standard,
    /// A browser-identical TLS handshake, for hosts that fingerprint it.
    /// Only reachable with the `tls-emulation` feature.
    Emulee,
}

/// Picks the stack for a URL, from what we have learned about its host.
///
/// Returns [`Pile::Standard`] unless the feature is on AND the host has a
/// recorded history of walling the direct route. A host we have never seen gets
/// the cheap client: paying for emulation on a first visit would spend the cost
/// on the 99% of hosts that never needed it.
pub fn pile_pour(url: &str) -> Pile {
    if !emulation_disponible() {
        return Pile::Standard;
    }
    let mure = crate::memoire_hotes::globales()
        .fiche(url)
        .is_some_and(|f| f.mure());
    if mure {
        Pile::Emulee
    } else {
        Pile::Standard
    }
}

/// Is a TLS-emulating client compiled into this binary?
///
/// A plain `cfg!`, exposed as a function so callers and tests can ask the
/// question without repeating the feature name.
pub const fn emulation_disponible() -> bool {
    cfg!(feature = "tls-emulation")
}

/// One line for the model when the emulating stack is used, so an unusual
/// route is never silent. Same rule as `memoire_hotes::note`.
pub fn note(pile: Pile) -> Option<&'static str> {
    match pile {
        Pile::Standard => None,
        Pile::Emulee => Some(
            "[browser-identical TLS handshake used: this host fingerprints the connection]",
        ),
    }
}

/// Fetch through the emulating stack.
///
/// Present only with `tls-emulation`. The caller is expected to fall back to
/// the ordinary chain when this returns `None`, exactly as it does for jina and
/// Wayback: a new transport must add a route, never become a single point of
/// failure.
#[cfg(feature = "tls-emulation")]
pub async fn recuperer_emule(url: &str) -> Option<String> {
    use wreq_util::Emulation;

    let client = wreq::Client::builder()
        .emulation(Emulation::Chrome136)
        .build()
        .ok()?;
    let reponse = client.get(url).send().await.ok()?;
    if !reponse.status().is_success() {
        return None;
    }
    reponse.text().await.ok()
}

/// Without the feature there is no emulating stack, and saying so plainly beats
/// a `cfg` at every call site.
#[cfg(not(feature = "tls-emulation"))]
pub async fn recuperer_emule(_url: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default build must not change behaviour, whatever the memory holds.
    #[test]
    fn sans_la_fonctionnalite_tout_passe_par_la_pile_standard() {
        if emulation_disponible() {
            return; // this test describes the default build
        }
        assert_eq!(pile_pour("https://n-importe.test/x"), Pile::Standard);
    }

    #[test]
    fn un_hote_jamais_vu_ne_paie_pas_lemulation() {
        // Never-seen hosts get the cheap client even with the feature on: the
        // cost belongs to the hosts that earned it.
        assert_eq!(pile_pour("https://jamais-croise.test/"), Pile::Standard);
    }

    #[tokio::test]
    async fn sans_la_fonctionnalite_la_route_emulee_est_un_non_evenement() {
        if emulation_disponible() {
            return;
        }
        assert!(
            recuperer_emule("https://exemple.test/").await.is_none(),
            "the default build must report no emulated route, not attempt one"
        );
    }

    #[test]
    fn la_pile_emulee_sannonce() {
        assert!(note(Pile::Standard).is_none(), "the ordinary route needs no note");
        assert!(note(Pile::Emulee).is_some(), "an unusual route must be visible");
    }
}
