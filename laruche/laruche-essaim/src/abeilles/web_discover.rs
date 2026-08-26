//! web_discover: find the URLs a site does NOT link to.
//!
//! Every fetch/search tool on the market optimizes TRANSPORT: beat Cloudflare,
//! impersonate Chrome, solve the JS challenge. That is the wrong bottleneck for
//! a whole class of sites. A 2016 HTTrack mirror with no protection at all can
//! still defeat every agent, because the problem is DISCOVERY: the link graph
//! is not reachable through `<a href>`.
//!
//! Reference case (`ds.lordtry.com`, the failure this tool was built from):
//! - the nav menu is `href="#"` + `onclick location='...'` → a static crawler
//!   sees a dead end and reports "nothing here";
//! - `/file/` ships an `index.html`, so Apache serves it INSTEAD of the
//!   `Index of` listing → directory enumeration is blind;
//! - the directory is named `file` (English singular) on a French site → a
//!   wordlist of `fichiers`/`files`/`download` misses by one word.
//!   Five `.dsparty` files sat there, live, unreachable by every tool tried.
//!
//! Six orthogonal channels, fused and verified. The first two never touch the
//! target's infrastructure at all, which is why they run first:
//! - [`Canal::Archive`]: the Wayback CDX index. Every URL the archive ever saw
//!   for the host, including files that are unlinked, renamed or deleted. One
//!   request, no anti-bot, no load on the target. Highest yield by far, and
//!   almost no agent tool uses it for DISCOVERY (only as a fetch fallback).
//! - [`Canal::Sousdomaines`]: Certificate Transparency logs. Every public TLS
//!   certificate is logged, so the sibling hosts of a domain are enumerable
//!   without asking the domain: staging, api, admin, and forgotten sites. On the
//!   reference domain this turns one fansite into sixteen.
//! - [`Canal::Plan`]: `robots.txt`, sitemaps, feeds - the map the publisher
//!   declared, including the `Disallow` paths they would rather not advertise.
//! - [`Canal::Liens`]: the real link graph - `<a>`, but also `<frame>`,
//!   `<iframe>`, `<area>`, and `location='...'` targets buried in JS menus.
//! - [`Canal::Listing`]: open directory indexes (Apache/nginx), recursed.
//! - [`Canal::Sondage`]: a wordlist DERIVED FROM THE SITE'S OWN TEXT, plus
//!   FR/EN singular and plural variants - not a generic dictionary.
//!
//! Fusion is the point. On the reference case `listing` is blind, `liens` needs
//! the JS menu read, and `archive` alone lands the five saves. No single channel
//! wins; the union does. Every candidate is then verified live, and the verdict
//! is honest: LIVE (confirmed 2xx), or GONE but retrievable from the archive.
//!
//! Deliberately NOT here: hydration payload extraction (`__NEXT_DATA__`,
//! `self.__next_f`, `__INITIAL_STATE__`). Measured on real Next.js sites, those
//! payloads yield build artifacts (`/_next/static/chunks/*.js`) and nothing the
//! anchors did not already carry, so they buy no DISCOVERY. They are a content
//! extraction technique, and they belong in `web_fetch`, which is the tool that
//! reads a page rather than the one that finds it.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::stream::{self, StreamExt};
use std::collections::{BTreeMap, HashSet};

/// Browser UA. The reference case returns 403 to anything else: the very first
/// wall an agent hits is usually a one-line User-Agent check, not Cloudflare.
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                  (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

const MAX_RESULTATS_DEFAUT: usize = 200;
const PLAFOND_RESULTATS: usize = 2_000;
/// Concurrent requests. Politeness over speed: this hits a single host.
const CONCURRENCE: usize = 8;
const TIMEOUT_SECS: u64 = 20;
/// Depth for recursive `Index of` descent.
const PROFONDEUR_LISTING: usize = 2;
/// Hard cap on wordlist probes, so `auto` never turns into a scan.
const MAX_SONDAGES: usize = 64;
/// Frames followed from the entry page (a frameset holds no content itself).
const MAX_FRAMES: usize = 12;
/// Archive entries listed in the "gone" section before collapsing to a count.
const MAX_ARCHIVE_AFFICHEE: usize = 40;

/// Where a candidate URL came from. Kept per-URL so the model can tell a
/// confirmed link from a guess.
///
/// Declaration order is confidence order: the sort surfaces what a publisher
/// declared or an index recorded before what a wordlist guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Canal {
    Archive,
    Plan,
    Liens,
    Listing,
    Sousdomaines,
    Sondage,
}

impl Canal {
    fn etiquette(self) -> &'static str {
        match self {
            Canal::Archive => "archive",
            Canal::Plan => "sitemap",
            Canal::Liens => "links",
            Canal::Listing => "listing",
            Canal::Sousdomaines => "subdomains",
            Canal::Sondage => "probe",
        }
    }
}

/// A candidate URL plus everything we learned about it.
#[derive(Debug, Clone)]
struct Trouvaille {
    url: String,
    canaux: Vec<Canal>,
    statut: Option<u16>,
    taille: Option<u64>,
    mime: Option<String>,
}

impl Trouvaille {
    fn nouveau(url: String, canal: Canal) -> Self {
        Self { url, canaux: vec![canal], statut: None, taille: None, mime: None }
    }

    /// Live and confirmed by a real request.
    fn vivant(&self) -> bool {
        matches!(self.statut, Some(c) if (200..300).contains(&c))
    }

    fn canaux_lisibles(&self) -> String {
        self.canaux
            .iter()
            .map(|c| c.etiquette())
            .collect::<Vec<_>>()
            .join("+")
    }
}

/// Discover URLs a site does not link to.
pub struct WebDiscover;

#[async_trait]
impl Abeille for WebDiscover {
    fn nom(&self) -> &str {
        "web_discover"
    }

    fn description(&self) -> &str {
        "Find files, pages and sibling hosts a site does NOT link to, that web_fetch and \
         web_search cannot reach: JS-only menus, framesets, directories with no index \
         listing, unlinked or deleted files, staging and forgotten subdomains. Fuses six \
         channels (Wayback CDX index, Certificate Transparency logs, robots/sitemaps/feeds, \
         real link graph incl. JS menus and frames, open directory listings, site-derived \
         wordlist probing) and VERIFIES every hit live. Use it when a site seems to have \
         nothing but you suspect otherwise, to list downloadable files, or to map a domain's \
         hosts. Filter with `ext` ('pdf,zip')."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "Site or directory URL to explore" },
                "mode": {
                    "type": "string",
                    "enum": ["auto", "archive", "sitemap", "links", "listing", "subdomains", "probe"],
                    "description": "auto (all channels, default) | archive (Wayback index only: fastest, no load on target) | sitemap (robots.txt, sitemaps and feeds: what the publisher declared) | links (link graph incl. JS menus and frames) | listing (open directory indexes) | subdomains (Certificate Transparency: staging, api, forgotten hosts) | probe (site-derived wordlist)"
                },
                "ext": { "type": "string", "description": "Comma-separated extension filter, e.g. 'pdf,zip,dsparty'. Empty = everything." },
                "max_results": { "type": "integer", "description": "Max URLs returned (default 200, max 2000)" },
                "verify": { "type": "boolean", "description": "Check each candidate live for status/size/type (default true)" }
            },
            "required": ["url"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let url_brut = args["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))?;
        // Secret substitution, as in web_fetch: the value never reaches the LLM.
        let url_sub = crate::secrets::substituer(url_brut);
        let cible = normaliser_entree(url_sub.as_str());

        if !cible.starts_with("http://") && !cible.starts_with("https://") {
            return Ok(ResultatAbeille::err("URL must start with http:// or https://"));
        }

        let mode = args["mode"].as_str().unwrap_or("auto").to_lowercase();
        let max_resultats = (args["max_results"]
            .as_u64()
            .unwrap_or(MAX_RESULTATS_DEFAUT as u64) as usize)
            .clamp(1, PLAFOND_RESULTATS);
        let verifier = args["verify"].as_bool().unwrap_or(true);
        let extensions = extensions_demandees(args["ext"].as_str().unwrap_or(""));

        let client = match reqwest::Client::builder()
            .user_agent(UA)
            .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
            .build()
        {
            Ok(c) => c,
            Err(e) => return Ok(ResultatAbeille::err(format!("HTTP client: {e}"))),
        };

        let canaux: Vec<Canal> = match mode.as_str() {
            "archive" => vec![Canal::Archive],
            "sitemap" => vec![Canal::Plan],
            "links" => vec![Canal::Liens],
            "listing" => vec![Canal::Listing],
            "subdomains" => vec![Canal::Sousdomaines],
            "probe" => vec![Canal::Sondage],
            _ => vec![
                Canal::Archive,
                Canal::Plan,
                Canal::Liens,
                Canal::Listing,
                Canal::Sousdomaines,
                Canal::Sondage,
            ],
        };

        // ── collection ──────────────────────────────────────────────────
        let mut trouvailles: BTreeMap<String, Trouvaille> = BTreeMap::new();
        let mut notes: Vec<String> = Vec::new();

        for canal in &canaux {
            let issue = match canal {
                Canal::Archive => canal_archive(&client, &cible).await,
                Canal::Plan => canal_plan(&client, &cible).await,
                Canal::Liens => canal_liens(&client, &cible).await,
                Canal::Listing => canal_listing(&client, &cible).await,
                Canal::Sousdomaines => canal_sousdomaines(&client, &cible).await,
                Canal::Sondage => canal_sondage(&client, &cible).await,
            };
            match issue {
                Ok(urls) => {
                    let avant = trouvailles.len();
                    for u in urls {
                        trouvailles
                            .entry(u.clone())
                            .and_modify(|t| {
                                if !t.canaux.contains(canal) {
                                    t.canaux.push(*canal);
                                }
                            })
                            .or_insert_with(|| Trouvaille::nouveau(u, *canal));
                    }
                    notes.push(format!(
                        "{}: +{} new",
                        canal.etiquette(),
                        trouvailles.len().saturating_sub(avant)
                    ));
                }
                // A dead channel must not sink the call: the others still carry.
                Err(e) => notes.push(format!("{}: failed ({e})", canal.etiquette())),
            }
        }

        if trouvailles.is_empty() {
            return Ok(ResultatAbeille::ok(format!(
                "web_discover on {cible}\nChannels: {}\n\nNo candidate URL found.",
                notes.join(" | ")
            )));
        }

        // ── extension filter ────────────────────────────────────────────
        let total_brut = trouvailles.len();
        let mut liste: Vec<Trouvaille> = trouvailles
            .into_values()
            .filter(|t| extensions.is_empty() || extension_correspond(&t.url, &extensions))
            .collect();

        // Corroborated candidates first, then by channel rank: the model should
        // read facts before hypotheses, and `probe` results are hypotheses.
        liste.sort_by(|a, b| {
            b.canaux
                .len()
                .cmp(&a.canaux.len())
                .then_with(|| a.canaux.iter().min().cmp(&b.canaux.iter().min()))
                .then_with(|| a.url.cmp(&b.url))
        });
        let tronque = liste.len() > max_resultats;
        liste.truncate(max_resultats);

        // ── verification ────────────────────────────────────────────────
        if verifier {
            liste = verifier_lot(&client, liste).await;
        }

        Ok(ResultatAbeille::ok(rendre(
            &cible,
            &liste,
            &notes,
            total_brut,
            tronque,
            verifier,
            &extensions,
        )))
    }
}

// ════════════════════════════════════════════════════════════════════════
// Channels
// ════════════════════════════════════════════════════════════════════════

/// The Wayback CDX index: every URL the archive ever saw for this host.
///
/// This is the channel that wins the reference case. It is a plain text index
/// query, so it costs one request, touches no anti-bot, and puts zero load on
/// the target. It also surfaces what the LIVE site no longer links or serves.
async fn canal_archive(client: &reqwest::Client, cible: &str) -> Result<Vec<String>> {
    let hote = hote_de(cible).ok_or_else(|| anyhow::anyhow!("no host in URL"))?;
    // Strip `www.` so the index covers both hostnames: the reference case stores
    // some files under `ds.lordtry.com` and others under `www.ds.lordtry.com`.
    let racine = hote.strip_prefix("www.").unwrap_or(&hote);
    let requete = format!(
        "https://web.archive.org/cdx/search/cdx?url={}*&fl=original&collapse=urlkey&limit=5000",
        urlencoding::encode(racine)
    );

    // The CDX endpoint drops requests under load, and losing this channel costs
    // the whole answer on a site whose links are unreachable. One retry, and a
    // budget larger than the default because the index can be slow on a big host.
    let mut derniere = None;
    let mut corps = None;
    for essai in 0..2 {
        if essai > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        match client
            .get(&requete)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
        {
            Ok(r) => match r.text().await {
                Ok(t) => {
                    corps = Some(t);
                    break;
                }
                Err(e) => derniere = Some(e),
            },
            Err(e) => derniere = Some(e),
        }
    }
    let corps = match corps {
        Some(c) => c,
        None => return Err(anyhow::anyhow!("CDX unreachable: {}", derniere.unwrap())),
    };

    let mut urls = Vec::new();
    for ligne in corps.lines() {
        let brut = ligne.split_whitespace().next().unwrap_or("").trim();
        if brut.is_empty() {
            continue;
        }
        // CDX stores the crawl-time URL (`http://host:80/path`). Bring it back
        // to something fetchable against the live site.
        let propre = brut.replace(":80/", "/").replacen("http://", "https://", 1);
        // The index mixes `host` and `www.host`. Pin every hit to the hostname
        // the caller gave us: it de-duplicates the same file seen under both,
        // and the bare host may not even hold a valid certificate.
        if let Some(canonique) = rehoter(&propre, &hote) {
            urls.push(canonique);
        }
    }
    Ok(urls)
}

/// Rewrite a URL's host, keeping scheme, path and query.
fn rehoter(url: &str, hote: &str) -> Option<String> {
    let (scheme, reste) = url.split_once("://")?;
    let chemin = match reste.find(['/', '?', '#']) {
        Some(i) => &reste[i..],
        None => "/",
    };
    Some(format!("{scheme}://{hote}{chemin}"))
}

/// What the publisher declared: `robots.txt`, sitemaps, syndication feeds.
///
/// This is the map the site WANTS read, so it is authoritative where it exists
/// and free where it does not. Note it does touch the target, unlike
/// [`canal_archive`]: robots and sitemaps live on the origin server.
async fn canal_plan(client: &reqwest::Client, cible: &str) -> Result<Vec<String>> {
    let racine = origine_de(cible).ok_or_else(|| anyhow::anyhow!("no origin in URL"))?;
    let mut urls = Vec::new();
    let mut plans: Vec<String> = Vec::new();

    // robots.txt names the sitemaps, including the ones on another host.
    if let Ok(rep) = client.get(format!("{racine}/robots.txt")).send().await {
        if rep.status().is_success() {
            if let Ok(texte) = rep.text().await {
                for ligne in texte.lines() {
                    let bas = ligne.trim().to_lowercase();
                    if let Some(reste) = bas.strip_prefix("sitemap:") {
                        // Re-slice the ORIGINAL line: lowercasing a URL can break
                        // a case-sensitive path.
                        let valeur = ligne.trim()[ligne.trim().len() - reste.trim().len()..].trim();
                        if let Some(u) = resoudre(&racine, valeur) {
                            plans.push(u);
                        }
                    }
                    // `Disallow:` names paths the publisher would rather hide,
                    // which makes them worth knowing about.
                    if let Some(reste) = bas.strip_prefix("disallow:") {
                        let chemin = reste.trim();
                        if chemin.len() > 1 && !chemin.contains('*') {
                            if let Some(u) = resoudre(&racine, chemin) {
                                urls.push(u);
                            }
                        }
                    }
                }
            }
        }
    }

    for defaut in EMPLACEMENTS_PLAN {
        plans.push(format!("{racine}{defaut}"));
    }
    plans.dedup();

    // One level of sitemap-index recursion: a `<sitemapindex>` lists sitemaps,
    // which list the URLs. Two hops is where the content actually is.
    let mut a_lire: Vec<String> = plans.into_iter().take(MAX_PLANS).collect();
    let mut lus: HashSet<String> = HashSet::new();
    let mut profondeur = 0;

    while !a_lire.is_empty() && profondeur <= 1 {
        let mut suivants = Vec::new();
        for plan in std::mem::take(&mut a_lire) {
            if !lus.insert(plan.clone()) {
                continue;
            }
            let Ok(rep) = client.get(&plan).send().await else {
                continue;
            };
            if !rep.status().is_success() {
                continue;
            }
            let Ok(corps) = rep.text().await else { continue };
            let index = corps.contains("<sitemapindex");
            for brut in extraire_balises(&corps, &["loc", "link", "guid"]) {
                let Some(u) = resoudre(&plan, &brut) else {
                    continue;
                };
                if index {
                    suivants.push(u);
                } else {
                    urls.push(u);
                }
            }
        }
        a_lire = suivants.into_iter().take(MAX_PLANS).collect();
        profondeur += 1;
    }

    urls.sort();
    urls.dedup();
    Ok(urls)
}

/// Conventional locations for sitemaps and feeds, tried when robots is silent.
const EMPLACEMENTS_PLAN: &[&str] = &[
    "/sitemap.xml",
    "/sitemap_index.xml",
    "/sitemap-index.xml",
    "/feed",
    "/rss.xml",
    "/atom.xml",
    "/index.xml",
];

/// Sitemaps or feeds read per level, so a huge index cannot run away with the call.
const MAX_PLANS: usize = 12;

/// Sibling hosts, from Certificate Transparency logs.
///
/// Every publicly trusted TLS certificate is logged, so the CT logs enumerate a
/// domain's hosts without touching its infrastructure: staging, api, admin, and
/// the sites a publisher forgot. On the reference domain this turns one fansite
/// into sixteen.
///
/// certspotter first (reliable JSON), crt.sh as fallback (frequently 502s).
async fn canal_sousdomaines(client: &reqwest::Client, cible: &str) -> Result<Vec<String>> {
    let hote = hote_de(cible).ok_or_else(|| anyhow::anyhow!("no host in URL"))?;
    let apex = domaine_apex(&hote);

    let mut noms: HashSet<String> = HashSet::new();

    let certspotter = format!(
        "https://api.certspotter.com/v1/issuances?domain={}&include_subdomains=true&expand=dns_names",
        urlencoding::encode(&apex)
    );
    if let Ok(rep) = client.get(&certspotter).send().await {
        if rep.status().is_success() {
            if let Ok(valeur) = rep.json::<serde_json::Value>().await {
                for entree in valeur.as_array().unwrap_or(&Vec::new()) {
                    for nom in entree["dns_names"].as_array().unwrap_or(&Vec::new()) {
                        if let Some(n) = nom.as_str() {
                            noms.insert(n.to_lowercase());
                        }
                    }
                }
            }
        }
    }

    if noms.is_empty() {
        let crtsh = format!(
            "https://crt.sh/?q={}&output=json",
            urlencoding::encode(&format!("%.{apex}"))
        );
        if let Ok(rep) = client.get(&crtsh).send().await {
            if rep.status().is_success() {
                if let Ok(valeur) = rep.json::<serde_json::Value>().await {
                    for entree in valeur.as_array().unwrap_or(&Vec::new()) {
                        for n in entree["name_value"].as_str().unwrap_or("").lines() {
                            noms.insert(n.trim().to_lowercase());
                        }
                    }
                }
            }
        }
    }

    if noms.is_empty() {
        return Err(anyhow::anyhow!("no CT log answered for {apex}"));
    }

    // A certificate carries every SAN it was issued for, and a shared hosting
    // cert lists hundreds of UNRELATED domains. Without this filter a query for
    // one site returns someone else's hosts.
    let suffixe = format!(".{apex}");
    let mut urls: Vec<String> = noms
        .into_iter()
        .filter(|n| *n == apex || n.ends_with(&suffixe))
        .filter(|n| !n.starts_with('*'))
        .map(|n| format!("https://{n}/"))
        .collect();
    urls.sort();
    Ok(urls)
}

/// Registrable domain, roughly. `www.ds.lordtry.com` → `lordtry.com`.
///
/// Two labels, or three when the second-to-last is a known second-level TLD.
/// Not a full public-suffix list: that would need a dependency and a periodic
/// refresh, for a gain limited to exotic suffixes.
fn domaine_apex(hote: &str) -> String {
    const DEUXIEME_NIVEAU: &[&str] = &[
        "co", "com", "net", "org", "gov", "edu", "ac", "gouv", "asso", "or", "ne",
    ];
    let labels: Vec<&str> = hote.split('.').filter(|l| !l.is_empty()).collect();
    match labels.len() {
        0..=2 => hote.to_string(),
        n => {
            let avant_dernier = labels[n - 2];
            let garde = if DEUXIEME_NIVEAU.contains(&avant_dernier) && n >= 3 { 3 } else { 2 };
            labels[n - garde..].join(".")
        }
    }
}

/// The real link graph: anchors, frames, and the JS menu targets a static
/// crawler cannot see.
async fn canal_liens(client: &reqwest::Client, cible: &str) -> Result<Vec<String>> {
    let html = client.get(cible).send().await?.text().await?;
    let mut urls = extraire_liens(&html, cible);

    // A frameset holds no content of its own: the pages that matter are one hop
    // away. Without this the reference case yields four URLs and nothing else.
    let frames: Vec<String> = urls
        .iter()
        .filter(|u| u.ends_with(".html") || u.ends_with(".htm"))
        .take(MAX_FRAMES)
        .cloned()
        .collect();
    for frame in frames {
        if let Ok(rep) = client.get(&frame).send().await {
            if let Ok(corps) = rep.text().await {
                urls.extend(extraire_liens(&corps, &frame));
            }
        }
    }

    urls.sort();
    urls.dedup();
    Ok(urls)
}

/// Open directory indexes, recursed.
async fn canal_listing(client: &reqwest::Client, cible: &str) -> Result<Vec<String>> {
    let mut urls = Vec::new();
    let mut a_visiter = vec![(repertoire_de(cible), 0usize)];
    let mut vus: HashSet<String> = HashSet::new();

    while let Some((repertoire, profondeur)) = a_visiter.pop() {
        if profondeur > PROFONDEUR_LISTING || !vus.insert(repertoire.clone()) {
            continue;
        }
        let Ok(rep) = client.get(&repertoire).send().await else {
            continue;
        };
        if !rep.status().is_success() {
            continue;
        }
        let Ok(html) = rep.text().await else { continue };
        if !est_listing(&html) {
            // A directory serving its own index.html hides the listing. That is
            // exactly how `/file/` stayed invisible, so read its links instead.
            urls.extend(extraire_liens(&html, &repertoire));
            continue;
        }
        for lien in extraire_liens(&html, &repertoire) {
            if lien.ends_with('/') {
                a_visiter.push((lien.clone(), profondeur + 1));
            }
            urls.push(lien);
        }
    }
    Ok(urls)
}

/// Wordlist probing, informed by the site's own vocabulary.
///
/// A generic dictionary is a lottery: the reference directory is `file`, English
/// singular, on a French site. Deriving candidates from the page text plus FR/EN
/// number variants beats a bigger blind list.
async fn canal_sondage(client: &reqwest::Client, cible: &str) -> Result<Vec<String>> {
    let racine = origine_de(cible).ok_or_else(|| anyhow::anyhow!("no origin in URL"))?;

    let mut mots: Vec<String> = MOTS_COURANTS.iter().map(|m| m.to_string()).collect();
    if let Ok(rep) = client.get(cible).send().await {
        if let Ok(html) = rep.text().await {
            mots.extend(mots_du_site(&html));
        }
    }
    // Both numbers for every candidate: `file` and `files`, `fichier` and
    // `fichiers`. One `s` was the whole gap on the reference case.
    let mut candidats: Vec<String> = Vec::new();
    for mot in mots {
        let sans_s = mot.trim_end_matches('s').to_string();
        for variante in [sans_s.clone(), format!("{sans_s}s")] {
            if variante.len() >= 2 && !candidats.contains(&variante) {
                candidats.push(variante);
            }
        }
    }
    candidats.truncate(MAX_SONDAGES);

    let trouves = stream::iter(candidats.into_iter().map(|mot| {
        let client = client.clone();
        let racine = racine.clone();
        async move {
            let url = format!("{racine}/{mot}/");
            match client.get(&url).send().await {
                Ok(r) if r.status().is_success() => Some(url),
                _ => None,
            }
        }
    }))
    .buffer_unordered(CONCURRENCE)
    .filter_map(|r| async move { r })
    .collect::<Vec<String>>()
    .await;

    Ok(trouves)
}

/// Directory names worth trying on any site, FR and EN, both numbers.
/// Deliberately short: the site-derived words carry most of the yield.
const MOTS_COURANTS: &[&str] = &[
    "file", "fichier", "download", "telechargement", "dl", "data", "doc", "media", "img",
    "image", "upload", "archive", "public", "static", "asset", "tmp", "save", "sauvegarde",
    "mod", "map", "carte", "outil", "tool", "bin", "backup", "divers", "ressource",
];

// ════════════════════════════════════════════════════════════════════════
// Verification
// ════════════════════════════════════════════════════════════════════════

/// Confirm every candidate against the live site, concurrently.
///
/// Without this the tool would report guesses as findings, which is the failure
/// mode it exists to remove. A candidate is LIVE only if a real request said so.
async fn verifier_lot(client: &reqwest::Client, liste: Vec<Trouvaille>) -> Vec<Trouvaille> {
    stream::iter(liste.into_iter().map(|mut t| {
        let client = client.clone();
        async move {
            // HEAD first; some hosts answer 405 to it, hence the ranged GET.
            let mut reponse = client.head(&t.url).send().await;
            let refuse = matches!(&reponse, Ok(r) if r.status().as_u16() == 405);
            if refuse || reponse.is_err() {
                reponse = client.get(&t.url).header("Range", "bytes=0-0").send().await;
            }
            if let Ok(r) = reponse {
                t.statut = Some(r.status().as_u16());
                t.taille = taille_annoncee(&r);
                t.mime = r
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|v| v.split(';').next().unwrap_or(v).trim().to_string());
            }
            t
        }
    }))
    .buffer_unordered(CONCURRENCE)
    .collect::<Vec<Trouvaille>>()
    .await
}

/// Size the server announces, for a HEAD or a ranged GET.
///
/// `Response::content_length()` reports the BODY length, which is 0 on a HEAD
/// and 1 on `Range: bytes=0-0`, so every file came back as "0B". Read the
/// headers instead: `Content-Length` on HEAD, and the total after the slash in
/// `Content-Range: bytes 0-0/8246` on the ranged GET.
fn taille_annoncee(reponse: &reqwest::Response) -> Option<u64> {
    let entetes = reponse.headers();
    if let Some(plage) = entetes
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
    {
        if let Some((_, total)) = plage.rsplit_once('/') {
            if let Ok(n) = total.trim().parse::<u64>() {
                return Some(n);
            }
        }
    }
    entetes
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())
        .or_else(|| reponse.content_length().filter(|n| *n > 0))
}

// ════════════════════════════════════════════════════════════════════════
// Parsing helpers (sync: no `scraper::Html` may live across an await)
// ════════════════════════════════════════════════════════════════════════

/// All navigable targets in a document, resolved against `base`.
///
/// Covers what a plain `a[href]` sweep misses: `<frame>`/`<iframe>` (the
/// reference case is a frameset), `<area>`, and `location='...'` inside the
/// JS handlers that back `href="#"` menus.
fn extraire_liens(html: &str, base: &str) -> Vec<String> {
    let mut sortie: Vec<String> = Vec::new();
    {
        use scraper::{Html, Selector};
        let doc = Html::parse_document(html);
        for (selecteur, attribut) in [
            ("a[href]", "href"),
            ("area[href]", "href"),
            ("frame[src]", "src"),
            ("iframe[src]", "src"),
        ] {
            let Ok(sel) = Selector::parse(selecteur) else {
                continue;
            };
            for element in doc.select(&sel) {
                if let Some(valeur) = element.value().attr(attribut) {
                    if let Some(u) = resoudre(base, valeur) {
                        sortie.push(u);
                    }
                }
            }
        }
    }
    for cible in cibles_javascript(html) {
        if let Some(u) = resoudre(base, &cible) {
            sortie.push(u);
        }
    }
    sortie.sort();
    sortie.dedup();
    sortie
}

/// Navigation targets hidden in inline JavaScript.
///
/// Scans for `location=`, `window.open(` and the Dreamweaver `MM_*` helpers,
/// then takes the next quoted string. Textual on purpose: no regex dependency,
/// and a menu built from `href="#"` + `onclick` is invisible without it.
fn cibles_javascript(html: &str) -> Vec<String> {
    const AMORCES: &[&str] = &[
        "location=",
        "location =",
        "location.href=",
        "location.href =",
        "window.open(",
        "MM_openBrWindow(",
        "MM_goToURL(",
    ];
    let mut sortie = Vec::new();
    for amorce in AMORCES {
        let mut reste = html;
        while let Some(pos) = reste.find(amorce) {
            let apres = &reste[pos + amorce.len()..];
            sortie.extend(
                litteraux_de_lappel(apres)
                    .into_iter()
                    .filter(|l| ressemble_a_un_chemin(l)),
            );
            reste = apres;
        }
    }
    sortie
}

/// Every quoted string in a JS call, up to the end of the statement.
///
/// Taking only the FIRST literal loses the reference case: `MM_goToURL` puts the
/// frame name first and the URL second (`('parent','file/index.html')`), while
/// `window.open` does the opposite. Collect them all and let
/// [`ressemble_a_un_chemin`] decide which one is a URL.
fn litteraux_de_lappel(texte: &str) -> Vec<String> {
    /// Characters scanned past the call opener before giving up.
    const FENETRE: usize = 300;

    let fin = texte
        .char_indices()
        .nth(FENETRE)
        .map(|(i, _)| i)
        .unwrap_or(texte.len());
    let zone = &texte[..fin];
    // Never run past the statement: the next one is a different call.
    let zone = match zone.find([';', '\n']) {
        Some(i) => &zone[..i],
        None => zone,
    };

    let mut litteraux = Vec::new();
    let mut reste = zone;
    while let Some(debut_guillemet) = reste.find(['\'', '"']) {
        let guillemet = reste[debut_guillemet..].chars().next().unwrap_or('"');
        let apres = &reste[debut_guillemet + guillemet.len_utf8()..];
        let Some(fin_valeur) = apres.find(guillemet) else {
            break;
        };
        let valeur = &apres[..fin_valeur];
        if !valeur.is_empty() && valeur.len() <= 512 {
            litteraux.push(valeur.to_string());
        }
        reste = &apres[fin_valeur + guillemet.len_utf8()..];
    }
    litteraux
}

/// Frame and window targets that sit in the same argument list as the URL.
const CIBLES_DE_FENETRE: &[&str] =
    &["parent", "_parent", "blank", "_blank", "self", "_self", "top", "_top", "new"];

/// Does this literal look like a path rather than a frame name or a template?
fn ressemble_a_un_chemin(valeur: &str) -> bool {
    let bas = valeur.trim().to_lowercase();
    if bas.is_empty() || CIBLES_DE_FENETRE.contains(&bas.as_str()) {
        return false;
    }
    // A JS concatenation (`'"+args[i+1]+"'`) is a template, not a URL.
    if bas.contains('+') || bas.contains(' ') || bas.contains('(') {
        return false;
    }
    // Either a path separator, or a file name with a plausible extension.
    // The bound is 8, not 4: `dsparty`, `torrent` and `sqlite3` are real
    // extensions, and a tight cap is how the reference case got missed once.
    bas.contains('/')
        || matches!(bas.rsplit_once('.'), Some((base, ext))
            if !base.is_empty()
                && (1..=8).contains(&ext.len())
                && ext.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// Text content of the named XML elements, in document order.
///
/// Hand-rolled instead of an XML parser: sitemaps and feeds only need `<loc>`
/// and `<link>`, the workspace carries no XML crate, and scraper's HTML parser
/// mangles self-closing Atom `<link href=...>`. Handles both the element form
/// (`<link>url</link>`, RSS) and the attribute form (`<link href="url"/>`, Atom).
fn extraire_balises(xml: &str, balises: &[&str]) -> Vec<String> {
    let mut sortie = Vec::new();
    for balise in balises {
        let ouvrante = format!("<{balise}");
        let fermante = format!("</{balise}>");
        let mut reste = xml;
        while let Some(pos) = reste.find(&ouvrante) {
            let apres = &reste[pos + ouvrante.len()..];
            let Some(fin_attributs) = apres.find('>') else {
                break;
            };
            let attributs = &apres[..fin_attributs];
            // Atom: the URL is in `href`, the element is empty.
            if let Some(href) = attributs.find("href=") {
                if let Some(valeur) = premier_attribut(&attributs[href + 5..]) {
                    sortie.push(valeur);
                }
            }
            let corps_debut = fin_attributs + 1;
            if let Some(fin) = apres[corps_debut..].find(&fermante) {
                let texte = apres[corps_debut..corps_debut + fin].trim();
                if !texte.is_empty() && !texte.contains('<') {
                    sortie.push(decoder_entites(texte));
                }
            }
            reste = &apres[corps_debut..];
        }
    }
    sortie
}

/// Value of a quoted XML attribute at the start of `texte`.
fn premier_attribut(texte: &str) -> Option<String> {
    let texte = texte.trim_start();
    let guillemet = texte.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let apres = &texte[guillemet.len_utf8()..];
    let fin = apres.find(guillemet)?;
    let valeur = apres[..fin].trim();
    (!valeur.is_empty()).then(|| decoder_entites(valeur))
}

/// The five predefined XML entities. Sitemaps escape `&` in query strings, and
/// an unescaped `&amp;` turns a valid URL into a 404.
fn decoder_entites(texte: &str) -> String {
    texte
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Apache/nginx/OVH autoindex signature.
fn est_listing(html: &str) -> bool {
    let bas = html.to_lowercase();
    bas.contains("<title>index of") || bas.contains("<h1>index of")
}

/// Lowercase word-ish tokens from a page, as probe candidates.
///
/// Pulls from anything alphabetic in the markup, which is where a site names its
/// own sections (`href="file/index.html"`, `alt="telechargement"`), minus the
/// HTML/JS vocabulary that would otherwise flood the list.
fn mots_du_site(html: &str) -> Vec<String> {
    let mut mots: HashSet<String> = HashSet::new();
    for segment in html
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
        .filter(|s| (3..=14).contains(&s.len()))
    {
        let bas = segment.to_lowercase();
        if bas.chars().all(|c| c.is_ascii_alphabetic()) && !STOPWORDS.contains(&bas.as_str()) {
            mots.insert(bas);
        }
    }
    let mut liste: Vec<String> = mots.into_iter().collect();
    liste.sort();
    liste
}

/// HTML/JS vocabulary that is never a directory name.
const STOPWORDS: &[&str] = &[
    "html", "head", "body", "title", "meta", "link", "href", "src", "img", "alt", "div",
    "span", "table", "tbody", "thead", "tfoot", "font", "size", "color", "align", "valign",
    "width", "height", "border", "cellpadding", "cellspacing", "class", "style", "type",
    "text", "content", "charset", "http", "https", "www", "com", "net", "org", "the", "and",
    "for", "with", "var", "function", "return", "document", "window", "true", "false",
    "null", "script", "javascript", "onclick", "onload", "onmouseover", "onmouseout",
    "target", "blank", "self", "parent", "frame", "frameset", "noframes", "center", "form",
    "input", "value", "name", "iso", "utf", "gif", "jpg", "jpeg", "png", "css", "top",
    "left", "right", "bottom", "middle", "nbsp", "quot", "amp", "new", "old", "page",
];

// ════════════════════════════════════════════════════════════════════════
// URL helpers (hand-rolled: the workspace carries no `url` crate)
// ════════════════════════════════════════════════════════════════════════

/// Add a scheme when the caller passed a bare host.
fn normaliser_entree(url: &str) -> String {
    let t = url.trim();
    if t.is_empty() || t.contains("://") {
        t.to_string()
    } else {
        format!("https://{t}")
    }
}

/// `https://host:port/a/b?q` → `host`.
fn hote_de(url: &str) -> Option<String> {
    let sans_scheme = url.split_once("://")?.1;
    let hote = sans_scheme.split(['/', '?', '#']).next().unwrap_or(sans_scheme);
    let hote = hote.split('@').next_back().unwrap_or(hote);
    let hote = hote.split(':').next().unwrap_or(hote);
    (!hote.is_empty()).then(|| hote.to_lowercase())
}

/// `https://host/a/b?q` → `https://host`.
fn origine_de(url: &str) -> Option<String> {
    let (scheme, reste) = url.split_once("://")?;
    let autorite = reste.split(['/', '?', '#']).next().unwrap_or(reste);
    (!autorite.is_empty()).then(|| format!("{scheme}://{autorite}"))
}

/// Directory the URL lives in, with a trailing slash.
fn repertoire_de(url: &str) -> String {
    let sans_requete = url.split(['?', '#']).next().unwrap_or(url);
    if sans_requete.ends_with('/') {
        return sans_requete.to_string();
    }
    let Some((scheme, reste)) = sans_requete.split_once("://") else {
        return sans_requete.to_string();
    };
    match reste.rfind('/') {
        // No slash past the authority: the URL IS the origin.
        None => format!("{sans_requete}/"),
        Some(pos) => format!("{scheme}://{}/", &reste[..pos]),
    }
}

/// Resolve `lien` against `base`, rejecting non-navigable schemes.
fn resoudre(base: &str, lien: &str) -> Option<String> {
    let lien = lien.trim();
    if lien.is_empty() || lien.starts_with('#') {
        return None;
    }
    let bas = lien.to_lowercase();
    for prefixe in ["javascript:", "mailto:", "tel:", "data:", "about:", "ftp:"] {
        if bas.starts_with(prefixe) {
            return None;
        }
    }
    // Apache autoindex column sorters (`?C=N;O=D`) are the same page, reordered.
    if lien.starts_with("?C=") {
        return None;
    }
    if bas.starts_with("http://") || bas.starts_with("https://") {
        return Some(lien.to_string());
    }
    let scheme = base.split_once("://")?.0;
    if let Some(reste) = lien.strip_prefix("//") {
        return Some(format!("{scheme}://{reste}"));
    }
    let origine = origine_de(base)?;
    if let Some(chemin) = lien.strip_prefix('/') {
        return Some(normaliser_chemin(&format!("{origine}/{chemin}")));
    }
    Some(normaliser_chemin(&format!("{}{lien}", repertoire_de(base))))
}

/// Collapse `.` and `..` segments so one file yields one URL, not three.
fn normaliser_chemin(url: &str) -> String {
    let Some((scheme, reste)) = url.split_once("://") else {
        return url.to_string();
    };
    let Some((autorite, chemin)) = reste.split_once('/') else {
        return url.to_string();
    };
    let (chemin, suffixe) = match chemin.find(['?', '#']) {
        Some(i) => (&chemin[..i], &chemin[i..]),
        None => (chemin, ""),
    };
    let mut pile: Vec<&str> = Vec::new();
    for segment in chemin.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                pile.pop();
            }
            s => pile.push(s),
        }
    }
    let fin = if chemin.ends_with('/') { "/" } else { "" };
    format!("{scheme}://{autorite}/{}{fin}{suffixe}", pile.join("/"))
}

/// Parse the `ext` filter into a lowercase, dot-free set.
fn extensions_demandees(brut: &str) -> Vec<String> {
    brut.split(',')
        .map(|e| e.trim().trim_start_matches('.').to_lowercase())
        .filter(|e| !e.is_empty())
        .collect()
}

/// Match on the file name only, so `/dsparty/index.html` is not a `.dsparty`.
fn extension_correspond(url: &str, extensions: &[String]) -> bool {
    let chemin = url.split(['?', '#']).next().unwrap_or(url);
    let fichier = chemin.rsplit('/').next().unwrap_or(chemin).to_lowercase();
    extensions.iter().any(|e| fichier.ends_with(&format!(".{e}")))
}

// ════════════════════════════════════════════════════════════════════════
// Rendering
// ════════════════════════════════════════════════════════════════════════

fn taille_lisible(octets: u64) -> String {
    match octets {
        0..=1023 => format!("{octets}B"),
        1024..=1_048_575 => format!("{:.1}K", octets as f64 / 1024.0),
        _ => format!("{:.1}M", octets as f64 / 1_048_576.0),
    }
}

/// Group by verdict so the model reads confirmed facts first, guesses last.
fn rendre(
    cible: &str,
    liste: &[Trouvaille],
    notes: &[String],
    total_brut: usize,
    tronque: bool,
    verifie: bool,
    extensions: &[String],
) -> String {
    let mut sortie = String::new();
    sortie.push_str(&format!("web_discover on {cible}\n"));
    sortie.push_str(&format!("Channels: {}\n", notes.join(" | ")));
    if !extensions.is_empty() {
        sortie.push_str(&format!(
            "Filter: .{} ({total_brut} candidates before filtering)\n",
            extensions.join(", .")
        ));
    }

    if !verifie {
        sortie.push_str(&format!("\nUNVERIFIED candidates ({}):\n", liste.len()));
        for t in liste {
            sortie.push_str(&format!("  {}  [{}]\n", t.url, t.canaux_lisibles()));
        }
        sortie.push_str("\nNote: `verify` was false. None of these were confirmed to exist.\n");
        return sortie;
    }

    let (vivants, morts): (Vec<_>, Vec<_>) = liste.iter().partition(|t| t.vivant());

    sortie.push_str(&format!("\nLIVE, confirmed ({}):\n", vivants.len()));
    for t in &vivants {
        let taille = t.taille.map(taille_lisible).unwrap_or_else(|| "?".into());
        let mime = t.mime.clone().unwrap_or_else(|| "?".into());
        sortie.push_str(&format!(
            "  {}  {taille}  {mime}  [{}]\n",
            t.url,
            t.canaux_lisibles()
        ));
    }
    if vivants.is_empty() {
        sortie.push_str("  (none)\n");
    }

    // Dead on the live site but seen by the archive: still retrievable, and this
    // is the only honest place to say so.
    let archives: Vec<&&Trouvaille> = morts
        .iter()
        .filter(|t| t.canaux.contains(&Canal::Archive))
        .collect();
    if !archives.is_empty() {
        sortie.push_str(&format!(
            "\nGONE from the live site, still in the archive ({}). Fetch via \
             https://web.archive.org/web/2016/<url>:\n",
            archives.len()
        ));
        for t in archives.iter().take(MAX_ARCHIVE_AFFICHEE) {
            let statut = t
                .statut
                .map(|s| s.to_string())
                .unwrap_or_else(|| "no answer".into());
            sortie.push_str(&format!("  {}  ({statut})\n", t.url));
        }
        if archives.len() > MAX_ARCHIVE_AFFICHEE {
            sortie.push_str(&format!(
                "  ... and {} more\n",
                archives.len() - MAX_ARCHIVE_AFFICHEE
            ));
        }
    }

    let autres = morts.len() - archives.len();
    if autres > 0 {
        sortie.push_str(&format!("\nDead, not in the archive: {autres} (dropped)\n"));
    }
    if tronque {
        sortie.push_str(&format!(
            "\nTruncated: {total_brut} candidates found, {} returned. Raise `max_results` \
             or narrow with `ext`.\n",
            liste.len()
        ));
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resoudre_gere_relatif_absolu_et_protocole() {
        let base = "https://ex.com/a/b/page.html";
        assert_eq!(resoudre(base, "c.html").unwrap(), "https://ex.com/a/b/c.html");
        assert_eq!(resoudre(base, "/d.html").unwrap(), "https://ex.com/d.html");
        assert_eq!(resoudre(base, "../e.html").unwrap(), "https://ex.com/a/e.html");
        assert_eq!(resoudre(base, "//cdn.io/f.js").unwrap(), "https://cdn.io/f.js");
        assert_eq!(resoudre(base, "https://x.io/g").unwrap(), "https://x.io/g");
    }

    #[test]
    fn resoudre_rejette_le_non_navigable() {
        let base = "https://ex.com/";
        for lien in ["#", "javascript:void(0)", "mailto:a@b.c", "?C=N;O=D", ""] {
            assert!(resoudre(base, lien).is_none(), "should reject {lien}");
        }
    }

    #[test]
    fn repertoire_de_remonte_au_dossier() {
        assert_eq!(repertoire_de("https://ex.com/a/b.html"), "https://ex.com/a/");
        assert_eq!(repertoire_de("https://ex.com/a/"), "https://ex.com/a/");
        assert_eq!(repertoire_de("https://ex.com"), "https://ex.com/");
    }

    #[test]
    fn hote_et_origine_survivent_au_port_et_au_chemin() {
        assert_eq!(hote_de("http://ds.lordtry.com:80/file/x").unwrap(), "ds.lordtry.com");
        assert_eq!(origine_de("https://ex.com/a?b=1").unwrap(), "https://ex.com");
    }

    /// The JS menu that hid `/file/index.html` on the reference site.
    #[test]
    fn cibles_javascript_lit_un_menu_href_diese() {
        // `r##` because the markup itself contains `"#`, which would close `r#`.
        let html = r##"<a href="#" onclick="MM_goToURL('parent','file/index.html');return false;">x</a>"##;
        let liens = extraire_liens(html, "https://ex.com/gauche.html");
        assert!(
            liens.contains(&"https://ex.com/file/index.html".to_string()),
            "JS target not recovered: {liens:?}"
        );
    }

    #[test]
    fn litteraux_de_lappel_collecte_tous_les_arguments() {
        let l = litteraux_de_lappel("'parent','file/index.html');return false;");
        assert_eq!(l, vec!["parent", "file/index.html"]);
        // The statement boundary stops the scan: the next call is not ours.
        let l = litteraux_de_lappel("'a.html'; location='b.html'");
        assert_eq!(l, vec!["a.html"]);
    }

    #[test]
    fn ressemble_a_un_chemin_trie_urls_frames_et_templates() {
        assert!(ressemble_a_un_chemin("file/index.html"));
        assert!(ressemble_a_un_chemin("temp.dsparty"));
        // Frame names travel in the same argument list as the URL.
        assert!(!ressemble_a_un_chemin("parent"));
        assert!(!ressemble_a_un_chemin("_blank"));
        // JS concatenation is a template, not a URL.
        assert!(!ressemble_a_un_chemin("\"+args[i+1]+\""));
        assert!(!ressemble_a_un_chemin(""));
    }

    #[test]
    fn extraire_liens_suit_les_frames() {
        let html = r#"<frameset><frame src="gauche.html"><frame name="c" src="premiere.html"></frameset>"#;
        let liens = extraire_liens(html, "https://ex.com/index.html");
        assert!(liens.contains(&"https://ex.com/gauche.html".to_string()));
        assert!(liens.contains(&"https://ex.com/premiere.html".to_string()));
    }

    #[test]
    fn est_listing_reconnait_apache() {
        assert!(est_listing("<html><head><title>Index of /maps</title>"));
        assert!(!est_listing("<html><head><title>Dungeon Siege</title>"));
    }

    #[test]
    fn filtre_extension_ne_confond_pas_le_dossier() {
        let exts = extensions_demandees(".dsparty, ZIP");
        assert_eq!(exts, vec!["dsparty", "zip"]);
        assert!(extension_correspond("https://e.com/f/temp.dsparty", &exts));
        assert!(extension_correspond("https://e.com/a.ZIP", &exts));
        assert!(!extension_correspond("https://e.com/dsparty/index.html", &exts));
    }

    #[test]
    fn mots_du_site_ecarte_le_vocabulaire_html() {
        let mots = mots_du_site(r#"<a href="file/index.html" class="menu">Telechargement</a>"#);
        assert!(mots.contains(&"file".to_string()));
        assert!(mots.contains(&"telechargement".to_string()));
        assert!(!mots.contains(&"href".to_string()));
        assert!(!mots.contains(&"class".to_string()));
    }

    #[test]
    fn domaine_apex_remonte_au_domaine_enregistrable() {
        assert_eq!(domaine_apex("www.ds.lordtry.com"), "lordtry.com");
        assert_eq!(domaine_apex("ds.lordtry.com"), "lordtry.com");
        assert_eq!(domaine_apex("lordtry.com"), "lordtry.com");
        // Known second-level TLDs need the third label.
        assert_eq!(domaine_apex("api.shop.co.uk"), "shop.co.uk");
        assert_eq!(domaine_apex("a.b.example.com.au"), "example.com.au");
    }

    #[test]
    fn extraire_balises_lit_sitemap_et_atom() {
        // RSS/sitemap: the URL is the element text.
        let sitemap = "<urlset><url><loc>https://ex.com/a?x=1&amp;y=2</loc></url>                       <url><loc>https://ex.com/b</loc></url></urlset>";
        let locs = extraire_balises(sitemap, &["loc"]);
        // The `&amp;` must be decoded or the URL 404s.
        assert_eq!(locs, vec!["https://ex.com/a?x=1&y=2", "https://ex.com/b"]);

        // Atom: the URL is the `href` attribute of an empty element.
        let atom = r#"<feed><entry><link href="https://ex.com/post-1"/></entry></feed>"#;
        assert_eq!(extraire_balises(atom, &["link"]), vec!["https://ex.com/post-1"]);
    }

    #[test]
    fn extraire_balises_ignore_un_corps_imbrique() {
        // A `<link>` wrapping markup is not a URL.
        let xml = "<link><span>x</span></link>";
        assert!(extraire_balises(xml, &["link"]).is_empty());
    }

    #[test]
    fn rehoter_epingle_lhote_sans_toucher_au_chemin() {
        // The CDX index mixes `host` and `www.host` for the same file.
        assert_eq!(
            rehoter("https://ds.lordtry.com/file/temp.dsparty", "www.ds.lordtry.com").unwrap(),
            "https://www.ds.lordtry.com/file/temp.dsparty"
        );
        assert_eq!(
            rehoter("https://ex.com", "www.ex.com").unwrap(),
            "https://www.ex.com/"
        );
    }

    #[test]
    fn normaliser_entree_ajoute_le_scheme() {
        assert_eq!(normaliser_entree("ex.com/a"), "https://ex.com/a");
        assert_eq!(normaliser_entree("https://ex.com"), "https://ex.com");
    }
}
