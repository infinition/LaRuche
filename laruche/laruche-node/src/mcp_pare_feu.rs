//! IP allowlist for the MCP server surface.
//!
//! That surface executes the whole tool registry, `shell_exec` and `file_write` included,
//! so it is the one place where "who is calling" deserves an answer before "what do they
//! want". The token check answers "do they know the secret"; this answers "are they even
//! supposed to be here", which is the cheaper question and the one that survives a leaked
//! token.
//!
//! Entries are plain addresses (`192.168.1.10`, `::1`) or CIDR blocks
//! (`192.168.1.0/24`, `fd00::/8`). Anything unparseable is IGNORED rather than treated as
//! a match: a typo must never widen the allowlist.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Is `ip` allowed by `liste`? An empty list allows nothing: turning the firewall on
/// without naming anyone means nobody, which is the safe reading of an empty allowlist and
/// the one that makes the mistake visible immediately instead of silently.
/// Compare two secrets without leaking their contents through timing. Overkill on a
/// loopback service, cheap enough not to argue about.
fn comparaison_constante(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub(crate) fn ip_autorisee(ip: IpAddr, liste: &[String]) -> bool {
    liste.iter().any(|entree| correspond(ip, entree.trim()))
}

fn correspond(ip: IpAddr, entree: &str) -> bool {
    if entree.is_empty() {
        return false;
    }
    // A convenience name for the common case, so the user does not have to remember that
    // loopback is two different addresses.
    if entree.eq_ignore_ascii_case("localhost") || entree.eq_ignore_ascii_case("loopback") {
        return ip.is_loopback();
    }
    match entree.split_once('/') {
        None => entree.parse::<IpAddr>().map(|a| a == ip).unwrap_or(false),
        Some((base, prefixe)) => {
            let Ok(base) = base.parse::<IpAddr>() else {
                return false;
            };
            let Ok(bits) = prefixe.parse::<u8>() else {
                return false;
            };
            match (base, ip) {
                (IpAddr::V4(b), IpAddr::V4(a)) if bits <= 32 => {
                    prefixe_commun(&b.octets(), &a.octets(), bits)
                }
                (IpAddr::V6(b), IpAddr::V6(a)) if bits <= 128 => {
                    prefixe_commun(&b.octets(), &a.octets(), bits)
                }
                // A v4 block never matches a v6 address, and the reverse: comparing them
                // would be the kind of near-miss that quietly lets someone in.
                _ => false,
            }
        }
    }
}

/// Do the first `bits` bits of both addresses agree?
fn prefixe_commun(a: &[u8], b: &[u8], bits: u8) -> bool {
    let entiers = (bits / 8) as usize;
    if a[..entiers] != b[..entiers] {
        return false;
    }
    let reste = bits % 8;
    if reste == 0 {
        return true;
    }
    let masque = 0xffu8 << (8 - reste);
    (a[entiers] & masque) == (b[entiers] & masque)
}

// ======================== Gate shared by both MCP surfaces ========================

/// Why a call was turned away, in the order the checks run. Cheapest first: a banned
/// address never reaches the config, and a blocked address never reaches the token.
pub(crate) enum Refus {
    /// Serving a ban; seconds left.
    Banni(u64),
    /// The server surface is switched off in Settings.
    Eteint,
    /// The address is not on the allowlist.
    HorsListe,
    /// Wrong or missing token (or non-loopback with no token configured).
    Jeton,
}

impl Refus {
    pub(crate) fn message(&self) -> String {
        match self {
            Refus::Banni(s) => format!("Too many refused calls: banned for another {s}s"),
            Refus::Eteint => "MCP server disabled: enable it in Settings".to_string(),
            Refus::HorsListe => "Address not in the MCP allowlist".to_string(),
            Refus::Jeton => {
                "Unauthorized (set LARUCHE_MCP_TOKEN, or call from localhost)".to_string()
            }
        }
    }
    /// Short label for the audit line and the Feed.
    pub(crate) fn motif(&self) -> &'static str {
        match self {
            Refus::Banni(_) => "banned",
            Refus::Eteint => "server off",
            Refus::HorsListe => "not allowlisted",
            Refus::Jeton => "bad token",
        }
    }
}

/// The whole door policy in one place, so `/mcp` and `/api/mcp` cannot drift apart. They
/// did: one honoured the Settings switch and the other did not, so turning the MCP server
/// off in the UI still left a surface serving the entire tool registry to any loopback
/// caller.
pub(crate) async fn controler(
    state: &crate::AppState,
    ip: Option<IpAddr>,
    jeton_recu: Option<&str>,
) -> Result<(), Refus> {
    let maintenant = Instant::now();
    // 1. Banned: answer nothing, cost nothing.
    if let Some(ip) = ip {
        if let Ok(mut v) = state.mcp_verrou.lock() {
            if let Verdict::Banni(reste) = v.verifier(ip, maintenant) {
                return Err(Refus::Banni(reste));
            }
        }
    }

    let (actif, pare_feu, autorisees, token_actif, token_attendu) = {
        let ec = state.essaim_config.read().await;
        (
            ec.mcp_server_actif,
            ec.mcp_pare_feu_actif,
            ec.mcp_ip_autorisees.clone(),
            ec.mcp_token_actif,
            ec.mcp_token.clone(),
        )
    };

    let echec = |refus: Refus| {
        if let (Some(ip), Ok(mut v)) = (ip, state.mcp_verrou.lock()) {
            v.echec(ip, maintenant);
        }
        Err(refus)
    };

    // 2. Off is off, on both surfaces.
    if !actif {
        return echec(Refus::Eteint);
    }
    // 3. Allowlist. A caller with no known address cannot be checked against a list, so
    //    when the firewall is on it is refused rather than waved through.
    if pare_feu && !ip.map(|a| ip_autorisee(a, &autorisees)).unwrap_or(false) {
        return echec(Refus::HorsListe);
    }
    // 4. Token, or loopback when no token is required.
    //
    // Order: the UI setting wins, then the environment, then loopback trust. The env var
    // stays supported so an existing deployment that set it keeps working, but the switch
    // in Settings is what a user can actually see and change.
    //
    // Comparison is length-then-bytes over the whole string rather than `==` short-circuit
    // semantics being relied on for secrecy; the token never appears in a refusal message.
    let attendu = if token_actif && !token_attendu.trim().is_empty() {
        Some(token_attendu)
    } else {
        std::env::var("LARUCHE_MCP_TOKEN").ok().filter(|t| !t.is_empty())
    };
    let autorise = match &attendu {
        Some(t) => jeton_recu.map(|r| comparaison_constante(r, t)).unwrap_or(false),
        // No token required: only this machine may call, and `shell_exec` is on the
        // other side of that decision.
        None => ip.map(|a| a.is_loopback()).unwrap_or(false),
    };
    if !autorise {
        return echec(Refus::Jeton);
    }

    if let (Some(ip), Ok(mut v)) = (ip, state.mcp_verrou.lock()) {
        v.succes(ip);
    }
    Ok(())
}

/// Record one MCP request, accepted or refused, with where it came from. This surface
/// executes `shell_exec`: a call arriving here and leaving no trace is the one thing that
/// must not happen. Goes to the activity log (audit panel, with the source address) and to
/// the Feed under its own kind, so it can be read and filtered like everything else.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn journaliser(
    state: &crate::AppState,
    ip: Option<IpAddr>,
    surface: &str,
    methode: &str,
    outil: Option<&str>,
    refus: Option<&Refus>,
    apercu: Option<String>,
) {
    let source = ip.map(|a| a.to_string()).unwrap_or_else(|| "?".into());
    let quoi = match outil {
        Some(o) => format!("{methode} {o}"),
        None => methode.to_string(),
    };
    let resume = match refus {
        Some(r) => format!("MCP {surface} from {source}: {quoi} REFUSED ({})", r.motif()),
        None => format!("MCP {surface} from {source}: {quoi}"),
    };
    crate::log_activite_riche(
        state,
        if refus.is_some() { "warn" } else { "info" },
        "mcp",
        resume.clone(),
        None,
        apercu,
        None,
        None,
    )
    .await;
    laruche_essaim::feed_journal::record(
        "MCP",
        "mcp",
        if refus.is_some() {
            "was refused"
        } else {
            "called"
        },
        format!("{quoi} ({source})"),
        chrono::Utc::now(),
    );
}

// ======================== Ban on repeated refusals ========================

/// How many refusals inside `FENETRE` before an address is banned.
const SEUIL: u32 = 5;
/// Refusals older than this stop counting: an address that failed twice last week is not
/// mid-attack.
const FENETRE: Duration = Duration::from_secs(60);
/// First ban length. It doubles on each repeat offence, up to `BAN_MAX`.
const BAN_BASE: Duration = Duration::from_secs(15 * 60);
const BAN_MAX: Duration = Duration::from_secs(24 * 60 * 60);
/// Hard cap on tracked addresses. Without it, spraying refusals from random source
/// addresses would grow this map without bound and turn the defence into the attack.
const MAX_SUIVIES: usize = 4096;

#[derive(Debug, Clone)]
struct Etat {
    echecs: u32,
    /// Start of the current counting window.
    depuis: Instant,
    banni_jusqu: Option<Instant>,
    /// Bans served so far, so a returning offender waits longer each time.
    bans: u32,
    /// Last touch, used to evict the stalest entry when the map is full.
    vu: Instant,
}

/// Refusal tracker for the MCP surface: too many rejected calls from one address and it
/// stops being answered at all. Guards against someone hammering the port to find a token
/// or simply to keep the node busy.
#[derive(Debug, Default)]
pub(crate) struct Verrou {
    suivies: HashMap<IpAddr, Etat>,
}

/// Why a call was let through or turned away. Kept separate from the HTTP layer so the
/// decision can be tested, logged and rendered without a socket in sight.
#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum Verdict {
    Autorise,
    /// Banned; the payload is the number of seconds left.
    Banni(u64),
}

impl Verrou {
    /// Is this address currently serving a ban? Call before doing any work at all: the
    /// point of the ban is to cost nothing.
    pub(crate) fn verifier(&mut self, ip: IpAddr, maintenant: Instant) -> Verdict {
        match self.suivies.get(&ip).and_then(|e| e.banni_jusqu) {
            Some(fin) if fin > maintenant => {
                Verdict::Banni((fin - maintenant).as_secs().max(1))
            }
            _ => Verdict::Autorise,
        }
    }

    /// Record a refused call. Returns the ban duration if this one tipped the address over.
    pub(crate) fn echec(&mut self, ip: IpAddr, maintenant: Instant) -> Option<Duration> {
        self.faire_de_la_place(maintenant);
        let e = self.suivies.entry(ip).or_insert(Etat {
            echecs: 0,
            depuis: maintenant,
            banni_jusqu: None,
            bans: 0,
            vu: maintenant,
        });
        e.vu = maintenant;
        // Window expired: this is a fresh burst, not a continuation of an old one.
        if maintenant.duration_since(e.depuis) > FENETRE {
            e.echecs = 0;
            e.depuis = maintenant;
        }
        e.echecs += 1;
        if e.echecs < SEUIL {
            return None;
        }
        // Each repeat doubles the wait, capped so a ban never becomes permanent by
        // accident: a misconfigured client deserves to be able to come back.
        let duree = BAN_BASE
            .saturating_mul(1u32 << e.bans.min(6))
            .min(BAN_MAX);
        e.bans += 1;
        e.echecs = 0;
        e.depuis = maintenant;
        e.banni_jusqu = Some(maintenant + duree);
        Some(duree)
    }

    /// A call that went through clears the counter: the window is about consecutive
    /// failures, not about a client that occasionally mistypes. The ban history is kept,
    /// so a repeat offender still escalates.
    pub(crate) fn succes(&mut self, ip: IpAddr) {
        if let Some(e) = self.suivies.get_mut(&ip) {
            e.echecs = 0;
            e.banni_jusqu = None;
        }
    }

    /// Currently banned addresses and their remaining seconds, for the audit panel.
    pub(crate) fn bannies(&self, maintenant: Instant) -> Vec<(IpAddr, u64)> {
        let mut v: Vec<(IpAddr, u64)> = self
            .suivies
            .iter()
            .filter_map(|(ip, e)| {
                e.banni_jusqu
                    .filter(|fin| *fin > maintenant)
                    .map(|fin| (*ip, (fin - maintenant).as_secs()))
            })
            .collect();
        v.sort_by_key(|e| std::cmp::Reverse(e.1));
        v
    }

    /// Lift a ban by hand, for when the banned client is your own laptop.
    pub(crate) fn liberer(&mut self, ip: IpAddr) -> bool {
        self.suivies.remove(&ip).is_some()
    }

    /// Drop expired entries, and if that is not enough, the stalest one. Bounded memory
    /// is what stops the tracker from being the vulnerability.
    fn faire_de_la_place(&mut self, maintenant: Instant) {
        if self.suivies.len() < MAX_SUIVIES {
            return;
        }
        self.suivies.retain(|_, e| {
            e.banni_jusqu.map(|f| f > maintenant).unwrap_or(false)
                || maintenant.duration_since(e.vu) <= FENETRE
        });
        while self.suivies.len() >= MAX_SUIVIES {
            // Un banni n'est JAMAIS evince pour faire de la place. Sans ce filtre, il
            // suffisait d'inonder le suivi d'adresses distinctes pour se faire oublier:
            // `min_by_key` prenait la plus ancienne sans regarder si elle etait bannie,
            // et quand les entrees partagent le meme horodatage - une rafale - l'ordre
            // d'un HashMap est aleatoire, donc la victime pouvait etre le banni.
            //
            // C'est ce que le test `la_pulverisation_ne_libere_pas_un_banni` verifie. Il
            // passait par chance, selon l'ordre de parcours du jour.
            let Some(plus_vieille) = self
                .suivies
                .iter()
                .filter(|(_, e)| !e.banni_jusqu.map(|f| f > maintenant).unwrap_or(false))
                .min_by_key(|(_, e)| e.vu)
                .map(|(ip, _)| *ip)
            else {
                // Plus rien d'evincable: tout ce qui reste est banni, et le rester est
                // le comportement voulu. La liste des bannis est bornee par le nombre
                // d'attaquants reels, pas par le trafic qu'ils produisent.
                break;
            };
            self.suivies.remove(&plus_vieille);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn adresse_exacte() {
        let liste = vec!["192.168.1.10".to_string()];
        assert!(ip_autorisee(ip("192.168.1.10"), &liste));
        assert!(!ip_autorisee(ip("192.168.1.11"), &liste));
    }

    #[test]
    fn bloc_cidr_v4() {
        let liste = vec!["192.168.1.0/24".to_string()];
        assert!(ip_autorisee(ip("192.168.1.1"), &liste));
        assert!(ip_autorisee(ip("192.168.1.255"), &liste));
        assert!(!ip_autorisee(ip("192.168.2.1"), &liste));
    }

    #[test]
    fn prefixe_non_aligne_sur_l_octet() {
        // /28 = 16 addresses: .16 through .31, and nothing either side.
        let liste = vec!["10.0.0.16/28".to_string()];
        assert!(ip_autorisee(ip("10.0.0.16"), &liste));
        assert!(ip_autorisee(ip("10.0.0.31"), &liste));
        assert!(!ip_autorisee(ip("10.0.0.15"), &liste));
        assert!(!ip_autorisee(ip("10.0.0.32"), &liste));
    }

    #[test]
    fn zero_bit_prend_tout_mais_seulement_sa_famille() {
        let liste = vec!["0.0.0.0/0".to_string()];
        assert!(ip_autorisee(ip("8.8.8.8"), &liste));
        assert!(!ip_autorisee(ip("::1"), &liste));
    }

    #[test]
    fn v4_et_v6_ne_se_melangent_pas() {
        assert!(!ip_autorisee(ip("::1"), &["127.0.0.1".to_string()]));
        assert!(!ip_autorisee(ip("127.0.0.1"), &["::1".to_string()]));
        assert!(!ip_autorisee(ip("::1"), &["192.168.0.0/16".to_string()]));
    }

    #[test]
    fn loopback_par_son_nom() {
        let liste = vec!["localhost".to_string()];
        assert!(ip_autorisee(ip("127.0.0.1"), &liste));
        assert!(ip_autorisee(ip("::1"), &liste));
        assert!(!ip_autorisee(ip("10.0.0.1"), &liste));
    }

    #[test]
    fn liste_vide_n_autorise_personne() {
        assert!(!ip_autorisee(ip("127.0.0.1"), &[]));
    }

    /// A typo must not widen the allowlist. It used to be tempting to treat an
    /// unparseable entry as a wildcard "let it through and let the token decide"; that
    /// turns one bad character into an open door.
    #[test]
    fn une_entree_illisible_n_autorise_rien() {
        for mauvaise in [
            "pas-une-ip",
            "192.168.1.0/33",
            "192.168.1.0/abc",
            "999.1.1.1",
            "",
            "   ",
        ] {
            assert!(
                !ip_autorisee(ip("192.168.1.1"), &[mauvaise.to_string()]),
                "entree {mauvaise:?} n'aurait pas du autoriser"
            );
        }
    }

    #[test]
    fn plusieurs_entrees_une_seule_suffit() {
        let liste = vec![
            "pas-une-ip".to_string(),
            "10.0.0.0/8".to_string(),
            "::1".to_string(),
        ];
        assert!(ip_autorisee(ip("10.5.5.5"), &liste));
        assert!(ip_autorisee(ip("::1"), &liste));
        assert!(!ip_autorisee(ip("172.16.0.1"), &liste));
    }

    #[test]
    fn ipv6_en_bloc() {
        let liste = vec!["fd00::/8".to_string()];
        assert!(ip_autorisee(ip("fd12:3456::1"), &liste));
        assert!(!ip_autorisee(ip("fe80::1"), &liste));
    }
    // ── Ban on repeated refusals ──────────────────────────────────

    #[test]
    fn sous_le_seuil_rien_ne_se_passe() {
        let mut v = Verrou::default();
        let t0 = Instant::now();
        for _ in 0..SEUIL - 1 {
            assert!(v.echec(ip("10.0.0.1"), t0).is_none());
        }
        assert_eq!(v.verifier(ip("10.0.0.1"), t0), Verdict::Autorise);
    }

    #[test]
    fn au_seuil_l_adresse_est_bannie() {
        let mut v = Verrou::default();
        let t0 = Instant::now();
        let mut duree = None;
        for _ in 0..SEUIL {
            duree = v.echec(ip("10.0.0.1"), t0);
        }
        assert_eq!(duree, Some(BAN_BASE));
        assert!(matches!(v.verifier(ip("10.0.0.1"), t0), Verdict::Banni(_)));
        // Une autre adresse n'est pas punie pour les fautes d'un voisin.
        assert_eq!(v.verifier(ip("10.0.0.2"), t0), Verdict::Autorise);
    }

    #[test]
    fn le_ban_expire() {
        let mut v = Verrou::default();
        let t0 = Instant::now();
        for _ in 0..SEUIL {
            v.echec(ip("10.0.0.1"), t0);
        }
        assert!(matches!(v.verifier(ip("10.0.0.1"), t0), Verdict::Banni(_)));
        let apres = t0 + BAN_BASE + Duration::from_secs(1);
        assert_eq!(v.verifier(ip("10.0.0.1"), apres), Verdict::Autorise);
    }

    #[test]
    fn les_echecs_espaces_ne_s_additionnent_pas() {
        let mut v = Verrou::default();
        let mut t = Instant::now();
        // Un echec par minute et demie: jamais dans la meme fenetre, jamais de ban.
        for _ in 0..(SEUIL * 3) {
            assert!(v.echec(ip("10.0.0.1"), t).is_none());
            t += FENETRE + Duration::from_secs(30);
        }
        assert_eq!(v.verifier(ip("10.0.0.1"), t), Verdict::Autorise);
    }

    #[test]
    fn le_recidiviste_attend_plus_longtemps() {
        let mut v = Verrou::default();
        let mut t = Instant::now();
        let mut durees = Vec::new();
        for _ in 0..3 {
            let mut d = None;
            for _ in 0..SEUIL {
                d = v.echec(ip("10.0.0.1"), t);
            }
            durees.push(d.unwrap());
            t += BAN_MAX + Duration::from_secs(1);
        }
        assert_eq!(durees[0], BAN_BASE);
        assert_eq!(durees[1], BAN_BASE * 2);
        assert_eq!(durees[2], BAN_BASE * 4);
    }

    #[test]
    fn un_succes_efface_le_compteur() {
        let mut v = Verrou::default();
        let t0 = Instant::now();
        for _ in 0..SEUIL - 1 {
            v.echec(ip("10.0.0.1"), t0);
        }
        v.succes(ip("10.0.0.1"));
        // Le compteur repart de zero: SEUIL-1 echecs de plus ne bannissent pas.
        for _ in 0..SEUIL - 1 {
            assert!(v.echec(ip("10.0.0.1"), t0).is_none());
        }
    }

    #[test]
    fn la_liberation_manuelle_leve_le_ban() {
        let mut v = Verrou::default();
        let t0 = Instant::now();
        for _ in 0..SEUIL {
            v.echec(ip("10.0.0.1"), t0);
        }
        assert!(v.liberer(ip("10.0.0.1")));
        assert_eq!(v.verifier(ip("10.0.0.1"), t0), Verdict::Autorise);
        assert!(!v.liberer(ip("10.0.0.1")));
    }

    /// Le compteur ne doit pas devenir l'attaque: quelqu'un qui pulverise des adresses
    /// sources differentes ne doit pas faire grossir la table sans fin.
    #[test]
    fn la_table_reste_bornee_sous_pulverisation() {
        let mut v = Verrou::default();
        let t0 = Instant::now();
        for i in 0..(MAX_SUIVIES + 500) {
            let a = std::net::Ipv4Addr::from(i as u32 + 0x0A00_0000);
            v.echec(IpAddr::V4(a), t0);
        }
        assert!(
            v.suivies.len() <= MAX_SUIVIES,
            "table a {} entrees, plafond {}",
            v.suivies.len(),
            MAX_SUIVIES
        );
    }

    /// Une adresse reellement bannie ne doit pas etre evincee par la pulverisation:
    /// ce serait le moyen le plus simple d'annuler son propre ban.
    #[test]
    fn la_pulverisation_ne_libere_pas_un_banni() {
        let mut v = Verrou::default();
        let t0 = Instant::now();
        let mechant = ip("203.0.113.7");
        for _ in 0..SEUIL {
            v.echec(mechant, t0);
        }
        assert!(matches!(v.verifier(mechant, t0), Verdict::Banni(_)));
        for i in 0..(MAX_SUIVIES + 200) {
            let a = std::net::Ipv4Addr::from(i as u32 + 0x0A00_0000);
            v.echec(IpAddr::V4(a), t0);
        }
        assert!(
            matches!(v.verifier(mechant, t0), Verdict::Banni(_)),
            "le banni a ete evince par la pulverisation"
        );
    }

    #[test]
    fn la_liste_des_bannis_sert_l_audit() {
        let mut v = Verrou::default();
        let t0 = Instant::now();
        for _ in 0..SEUIL {
            v.echec(ip("10.0.0.1"), t0);
        }
        let bannies = v.bannies(t0);
        assert_eq!(bannies.len(), 1);
        assert_eq!(bannies[0].0, ip("10.0.0.1"));
        assert!(bannies[0].1 > 0);
        // Une fois le ban passe, la liste est vide.
        assert!(v.bannies(t0 + BAN_BASE + Duration::from_secs(1)).is_empty());
    }
}
