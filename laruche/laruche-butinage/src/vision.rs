//! On envoie les captures au modele, et on encaisse proprement son refus.
//!
//! Une capture d'outil part maintenant vers le modele, pas seulement vers
//! l'ecran: sans ca il annonce "capture prise" sans avoir rien regarde. Mais
//! tous les endpoints n'acceptent pas une image, y compris pour un modele dont
//! la fiche annonce la vision native: observe sur `deepseek-v4-flash`, qui
//! repond 400 `This model does not support image` sur une simple capture de
//! navigateur, envoyee au format OpenAI standard (`image_url` + data URL).
//!
//! Avant ce module, ce refus tuait le tour entier. C'etait la pire des issues:
//! l'agent mourait au milieu de son travail a cause d'une photo dont il
//! n'avait pas besoin pour continuer.
//!
//! Donc: on envoie. Si le fournisseur refuse, on retient le modele, on retire
//! l'image, on previent le modele qu'elle existe sans qu'il la voie, et le tour
//! continue. Une fois par modele et par session, pas davantage.
//!
//! Aucune liste de modeles ici, ni blanche ni noire, et c'est une decision.
//! Deviner d'apres le nom se trompe dans les deux sens, et la fiche technique
//! n'est pas une preuve: seul compte ce que l'endpoint accepte vraiment.
//!
//! `LARUCHE_VISION=0` coupe l'envoi partout, `=1` le force malgre un refus
//! deja retenu.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Duree pendant laquelle un modele reste ecarte apres un refus.
///
/// La premiere version n'avait pas de duree: un refus valait pour toute la
/// session. Ca a fait exactement le degat qu'il fallait eviter. Une capture
/// d'ecran, plus lourde qu'une image collee a la main, se faisait refuser
/// pendant un pilotage; le modele etait range au placard; et le chat, qui
/// marchait tres bien avec des images plus legeres, cessait de fonctionner
/// jusqu'au redemarrage. Un seul echec sur un chemin condamnait tous les autres.
///
/// Dix minutes: assez pour ne pas repayer l'aller-retour a chaque message
/// pendant qu'on travaille, assez court pour que la situation se repare seule
/// sans que personne n'ait a redemarrer quoi que ce soit.
const REPIT: Duration = Duration::from_secs(600);

/// Modeles ecartes, avec l'heure a laquelle ils l'ont ete.
fn aveugles() -> &'static Mutex<HashMap<String, Instant>> {
    static S: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Le fournisseur vient de refuser une image: ne plus lui en envoyer, un temps.
pub fn marquer_aveugle(modele: &str) {
    if let Ok(mut s) = aveugles().lock() {
        s.insert(modele.to_lowercase(), Instant::now());
    }
    // A voix haute, et dans le JOURNAL. Un message dans le fil de discussion
    // defile et se perd; c'est apres coup qu'on cherche pourquoi un modele de
    // vision jure ne pas voir, et il faut alors une trace qui ait survecu.
    tracing::warn!(
        modele = %modele,
        repit_s = REPIT.as_secs(),
        "vision: plus aucune image envoyee a ce modele, il en a refuse une"
    );
}

/// Depuis combien de temps ce modele est-il ecarte, et pour combien encore.
///
/// Rend `None` s'il ne l'est pas. Sert a le DIRE: l'etat n'existait que dans une
/// table privee, donc personne ne pouvait constater qu'un modele avait ete raye
/// ni savoir quand il redeviendrait normal.
pub fn ecarte_depuis(modele: &str) -> Option<(u64, u64)> {
    let m = modele.to_lowercase();
    let t = aveugles().lock().ok()?.get(&m).copied()?;
    let ecoule = t.elapsed();
    if ecoule >= REPIT {
        return None;
    }
    Some((ecoule.as_secs(), (REPIT - ecoule).as_secs()))
}

/// Tous les modeles ecartes en ce moment: (modele, secondes restantes).
pub fn ecartes() -> Vec<(String, u64)> {
    let Ok(s) = aveugles().lock() else {
        return Vec::new();
    };
    s.iter()
        .filter_map(|(m, t)| {
            let e = t.elapsed();
            (e < REPIT).then(|| (m.clone(), (REPIT - e).as_secs()))
        })
        .collect()
}

/// Rend sa vue a un modele, tout de suite.
///
/// Sans cela, la seule issue etait d'attendre le repit ou de redemarrer le
/// noeud avec `LARUCHE_VISION=1`, c'est-a-dire de connaitre une variable
/// d'environnement dont rien ne parle nulle part.
pub fn reessayer(modele: &str) -> bool {
    let m = modele.to_lowercase();
    let mut trouve = false;
    if let Ok(mut s) = aveugles().lock() {
        trouve = s.remove(&m).is_some();
    }
    if let Ok(mut s) = serres().lock() {
        s.remove(&m);
    }
    if trouve {
        tracing::info!(modele = %modele, "vision: le modele recoit de nouveau les images");
    }
    trouve
}

/// La duree du repit, pour que l'interface annonce la bonne.
pub fn repit_secs() -> u64 {
    REPIT.as_secs()
}

/// Le corps d'erreur dit-il que le fournisseur ne veut pas de notre image?
///
/// Chaque fournisseur le formule a sa facon, d'ou la reconnaissance par
/// morceaux plutot que par phrase exacte. Deux familles:
///
/// **Le refus franc.** "This model does not support image". Rien a discuter.
///
/// **Le refus deguise en erreur de syntaxe.** Observe sur DeepSeek:
///
/// ```text
/// Failed to parse the request body as JSON:
///   messages[1].content[1].image_url.url: EOF while parsing a string
///   at line 1 column 181859
/// ```
///
/// Ca ressemble a un corps corrompu, et c'est ainsi qu'on le traitait: on
/// retentait a l'identique, trois fois, avant de tuer le tour. Mais le corps
/// etait intact, verifie octet par octet, base64 propre, JSON valide. Et
/// surtout le fournisseur NOMME le chemin fautif, donc il a lu la structure
/// sans probleme: seul le contenu de `image_url.url` le gene, c'est-a-dire
/// notre data URL. La meme image passe sans broncher sur un autre endpoint.
///
/// D'ou la regle: une erreur qui designe `image_url` est un probleme d'image,
/// pas de transport. Retenter sans elle a un sens, retenter a l'identique non.
pub fn corps_refuse_image(corps: &str) -> bool {
    !matches!(lire_refus(corps), Refus::Aucun)
}

/// Ce que le corps d'erreur nous apprend vraiment sur l'image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Refus {
    /// Rien a voir avec l'image.
    Aucun,
    /// "This model does not support image": le modele ne voit pas, point.
    Certain,
    /// Le fournisseur bute sur `image_url` sans dire pourquoi. Ca peut etre une
    /// limite de taille de son passage plutot qu'une absence de vision.
    Douteux,
}

/// Distingue le refus franc du refus douteux.
///
/// La distinction n'est pas de la finesse gratuite, c'est une erreur que j'ai
/// commise: traiter les deux pareil revenait a declarer aveugle, pour toute la
/// session, un modele de vision qui butait en realite sur la TAILLE de l'image.
/// Le premier envoi echouait, le modele etait raye, et tous les suivants
/// partaient sans image avec un message d'excuse. Impossible a diagnostiquer
/// pour la personne en face: elle voit un modele de vision qui jure ne pas voir.
///
/// Donc: le refus franc raye le modele tout de suite, le refus douteux ne fait
/// que resserrer le gabarit. On ne raye qu'apres avoir reessaye plus petit.
pub fn lire_refus(corps: &str) -> Refus {
    let c = corps.to_lowercase();
    // Un souci de FORMAT n'est pas une absence de vision: le modele voit, c'est
    // notre encodage qui ne lui plait pas.
    if c.contains("image format") {
        return Refus::Aucun;
    }
    if c.contains("does not support image")
        || c.contains("not support images")
        || c.contains("does not support vision")
        || c.contains("no vision support")
        || c.contains("invalid content type: image")
        || (c.contains("image") && c.contains("not supported"))
    {
        return Refus::Certain;
    }
    // "messages[1].content[1].image_url.url: EOF while parsing a string at
    // line 1 column 181859", ou la colonne est la taille exacte du corps. Le
    // fournisseur a lu la structure, donc son parseur va bien: il bute sur la
    // valeur. Une limite de taille non documentee explique tout aussi bien
    // qu'une absence de vision, et elle, on sait la contourner.
    if c.contains("image_url") {
        return Refus::Douteux;
    }
    Refus::Aucun
}

/// Modeles a qui l'on envoie desormais des images reduites au minimum.
fn serres() -> &'static Mutex<HashSet<String>> {
    static S: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Faut-il envoyer a ce modele des images au gabarit reduit?
pub fn budget_serre(modele: &str) -> bool {
    serres()
        .lock()
        .map(|s| s.contains(&modele.to_lowercase()))
        .unwrap_or(false)
}

/// Rend au modele sa deuxieme chance, pour les tests.
#[cfg(test)]
fn oublier(modele: &str) {
    let m = modele.to_lowercase();
    if let Ok(mut s) = aveugles().lock() {
        s.remove(&m);
    }
    if let Ok(mut s) = serres().lock() {
        s.remove(&m);
    }
}

/// Ce qu'il faut faire apres un refus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Suite {
    /// Rien a voir avec l'image: laisser la meteo faire son travail.
    Ignorer,
    /// Reessayer avec une image nettement plus petite.
    Retrecir,
    /// Reessayer sans image, et ne plus jamais en envoyer a ce modele.
    Renoncer,
}

/// Enregistre un refus et dit quoi faire ensuite.
///
/// Un refus douteux coute un essai en gabarit reduit. Le deuxieme sur le meme
/// modele tranche: ce n'etait pas la taille.
pub fn enregistrer_refus(modele: &str, corps: &str) -> Suite {
    match lire_refus(corps) {
        Refus::Aucun => Suite::Ignorer,
        Refus::Certain => {
            marquer_aveugle(modele);
            Suite::Renoncer
        }
        Refus::Douteux => {
            let m = modele.to_lowercase();
            let deja = serres().lock().map(|s| s.contains(&m)).unwrap_or(false);
            if deja {
                marquer_aveugle(modele);
                Suite::Renoncer
            } else {
                if let Ok(mut s) = serres().lock() {
                    s.insert(m);
                }
                Suite::Retrecir
            }
        }
    }
}

/// Envoie-t-on les images a ce modele?
///
/// Oui, tant qu'il n'a pas refuse dans cette session.
pub fn modele_voit(modele: &str) -> bool {
    match std::env::var("LARUCHE_VISION").as_deref() {
        Ok("0") | Ok("off") | Ok("false") => return false,
        Ok("1") | Ok("on") | Ok("true") => return true,
        _ => {}
    }
    let m = modele.to_lowercase();
    let ecarte = aveugles()
        .lock()
        .map(|s| s.get(&m).map(|t| t.elapsed() < REPIT).unwrap_or(false))
        .unwrap_or(false);
    !ecarte
}

/// Ce qu'on dit au modele a la place de l'image.
///
/// Le silence serait pire que l'erreur: l'outil vient d'annoncer "capture
/// prise, decris ce que tu vois", et un modele qui ne recoit rien decrit quand
/// meme, de memoire ou d'imagination. La phrase doit donc etre explicite sur
/// les trois points: l'image existe, l'utilisateur la voit, toi non.
pub fn note_sans_vision(combien: usize) -> String {
    format!(
        "\n\n[{combien} image(s) produced and displayed to the USER, but this model has no vision \
         and they were NOT sent to you. You cannot see them. Do not describe them and do not guess \
         what they contain: say plainly that you cannot see the image, and get the information \
         from text instead (browser read for a page, the accessibility tree for a window).]"
    )
}

/// Nombre d'images qui accompagnent une requete, au maximum.
///
/// Un agent qui pilote un navigateur prend une capture a chaque etape. Sans
/// borne, l'iteration numero vingt reexpedie les vingt captures, et le corps
/// enfle a chaque tour alors que dix-neuf de ces images ne servent plus a rien:
/// ce qui compte, c'est l'ecran MAINTENANT, et de quoi voir ce qui vient de
/// changer. Trois suffisent pour ca, et evitent de payer vingt fois une image
/// que le modele a deja utilisee.
pub const IMAGES_PAR_REQUETE: usize = 3;

/// Ne garde que les dernieres images d'une conversation deja convertie.
///
/// Rend le nombre d'images retirees.
pub fn borner_images(msgs: &mut [serde_json::Value], garde: usize) -> usize {
    // A l'envers: les plus recentes sont celles qu'on garde, et on ne sait
    // lesquelles qu'en partant de la fin.
    let mut vues = 0usize;
    let mut retirees = 0usize;
    for m in msgs.iter_mut().rev() {
        let combien = m
            .get("images")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        if combien == 0 {
            continue;
        }
        if vues + combien <= garde {
            vues += combien;
            continue;
        }
        retirees += depouiller(m, &note_trop_ancienne);
    }
    retirees
}

/// Retire les images d'une conversation deja convertie pour le fournisseur.
///
/// On travaille sur le JSON sortant plutot que sur l'historique: l'historique
/// est partage, il survit au tour, et il doit garder l'image pour le jour ou le
/// meme carnet repart sur un modele qui voit.
///
/// Rend le nombre d'images retirees.
pub fn retirer_images(msgs: &mut [serde_json::Value]) -> usize {
    let mut total = 0usize;
    for m in msgs.iter_mut() {
        total += depouiller(m, &note_sans_vision);
    }
    total
}

/// Ce qu'on dit d'une image trop ancienne pour repartir.
///
/// La formulation compte: dire "je ne vois pas" a un modele qui voit
/// parfaitement les captures recentes le rendrait incoherent, et il finirait
/// par douter de celles qu'il a sous les yeux.
pub fn note_trop_ancienne(combien: usize) -> String {
    format!(
        "

[{combien} image(s) from this earlier step, not re-sent to keep the request small.          You saw them at the time; rely on what you noted then, or take a fresh capture if you          need to look again.]"
    )
}

/// Retire les images d'UN message et pose la mention a leur place.
fn depouiller(m: &mut serde_json::Value, note: &dyn Fn(usize) -> String) -> usize {
    {
        let combien = m
            .get("images")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        // `attachments` porte aussi les pieces non-image (audio, fichiers): on
        // n'enleve que ce qui est une image, le reste ne genait personne.
        if let Some(att) = m.get_mut("attachments").and_then(|v| v.as_array_mut()) {
            att.retain(|p| {
                p.get("mime")
                    .or_else(|| p.get("mime_type"))
                    .and_then(|v| v.as_str())
                    .map(|s| !s.starts_with("image/"))
                    .unwrap_or(true)
            });
            let vide = att.is_empty();
            if vide {
                if let Some(o) = m.as_object_mut() {
                    o.remove("attachments");
                }
            }
        }
        if combien == 0 {
            return 0;
        }
        if let Some(o) = m.as_object_mut() {
            o.remove("images");
        }
        let texte = note(combien);
        let neuf = match m.get("content").and_then(|v| v.as_str()) {
            Some(c) => format!("{c}{texte}"),
            None => texte.trim().to_string(),
        };
        m["content"] = serde_json::Value::String(neuf);
        combien
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Les tests touchent une variable d'environnement, donc un seul a la fois.
    fn verrou() -> std::sync::MutexGuard<'static, ()> {
        static V: OnceLock<Mutex<()>> = OnceLock::new();
        V.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn on_envoie_a_tout_le_monde_jusqua_preuve_du_contraire() {
        let _g = verrou();
        std::env::remove_var("LARUCHE_VISION");
        // Aucun nom ne perd la vision sans que le fournisseur l'ait dit: un
        // modele dont une version voit et l'autre non porte le meme prefixe, et
        // deviner reviendrait a aveugler des modeles capables en silence.
        assert!(modele_voit("deepseek-v4-flash"));
        assert!(modele_voit("qwen2.5-coder:32b"));
        assert!(modele_voit("un-modele-de-2027"));
    }

    #[test]
    fn le_refus_du_fournisseur_est_retenu() {
        let _g = verrou();
        std::env::remove_var("LARUCHE_VISION");
        assert!(modele_voit("modele-cobaye"));
        marquer_aveugle("Modele-Cobaye"); // la casse ne doit pas sauver le modele
        assert!(!modele_voit("modele-cobaye"));
        // Le forcage passe outre: si le refus a ete mal interprete, la personne
        // doit pouvoir le dire.
        std::env::set_var("LARUCHE_VISION", "1");
        assert!(modele_voit("modele-cobaye"));
        std::env::remove_var("LARUCHE_VISION");
    }

    #[test]
    fn un_refus_douteux_retrecit_avant_de_renoncer() {
        let _g = verrou();
        std::env::remove_var("LARUCHE_VISION");
        let corps = "Failed to parse the request body as JSON:                      messages[1].content[1].image_url.url: EOF while parsing a string";
        // Premier refus douteux: on ne raye pas le modele, on retrecit. C'est
        // toute la difference entre un modele de vision qu'on aide a passer et
        // un modele de vision qu'on declare aveugle a tort.
        assert_eq!(enregistrer_refus("modele-vision", corps), Suite::Retrecir);
        assert!(modele_voit("modele-vision"), "pas encore raye");
        assert!(budget_serre("modele-vision"));
        // Deuxieme: ce n'etait pas la taille.
        assert_eq!(enregistrer_refus("modele-vision", corps), Suite::Renoncer);
        assert!(!modele_voit("modele-vision"));
    }

    #[test]
    fn un_modele_ecarte_retrouve_sa_chance() {
        let _g = verrou();
        std::env::remove_var("LARUCHE_VISION");
        oublier("modele-du-repit");
        marquer_aveugle("modele-du-repit");
        assert!(!modele_voit("modele-du-repit"));
        // On triche sur l'horloge en reculant la date du refus: le modele doit
        // revenir de lui-meme, sans redemarrage. C'est tout l'interet du repit,
        // et c'est ce qui manquait quand un pilotage rate condamnait le chat.
        if let Ok(mut s) = aveugles().lock() {
            s.insert(
                "modele-du-repit".into(),
                Instant::now() - REPIT - Duration::from_secs(1),
            );
        }
        assert!(modele_voit("modele-du-repit"));
    }

    #[test]
    fn un_refus_franc_ne_perd_pas_de_temps() {
        let _g = verrou();
        std::env::remove_var("LARUCHE_VISION");
        assert_eq!(
            enregistrer_refus("modele-texte", "This model does not support image"),
            Suite::Renoncer
        );
        assert!(!modele_voit("modele-texte"));
        assert!(!budget_serre("modele-texte"), "inutile de retrecir pour rien");
    }

    #[test]
    fn on_reconnait_le_refus_dans_le_corps() {
        assert!(corps_refuse_image(
            r#"{"error":{"message":"This model does not support image","type":"invalid_request_error"}}"#
        ));
        // Le refus deguise en erreur de syntaxe, mot pour mot celui de DeepSeek
        // sur une image de 254x254 dont le corps etait intact.
        assert!(corps_refuse_image(
            "Failed to parse the request body as JSON: messages[1].content[1].image_url.url: \
             EOF while parsing a string at line 1 column 181859"
        ));
        assert!(!corps_refuse_image("unsupported image format: bmp"));
        assert!(!corps_refuse_image("rate limit exceeded"));
        // Un corps vraiment corrompu, sans image en cause, reste un probleme de
        // transport: le retenter a l'identique a du sens, couper la vision non.
        assert!(!corps_refuse_image(
            "Failed to parse the request body as JSON: EOF while parsing a string \
             at line 1 column 102271"
        ));
    }

    #[test]
    fn retirer_laisse_le_texte_et_previent_le_modele() {
        let mut msgs = vec![
            json!({"role":"user","content":"regarde","images":["AAAA","BBBB"],
                   "attachments":[{"kind":"image","mime":"image/png","data":"AAAA"},
                                  {"kind":"audio","mime":"audio/wav","data":"CCCC"}]}),
            json!({"role":"assistant","content":"ok"}),
        ];
        assert_eq!(retirer_images(&mut msgs), 2);
        assert!(msgs[0].get("images").is_none());
        let c = msgs[0]["content"].as_str().unwrap();
        assert!(c.starts_with("regarde"), "le texte de l'utilisateur reste");
        assert!(c.contains("cannot see"), "le modele doit etre prevenu");
        // L'audio n'a rien a voir avec la vision et ne doit pas partir avec.
        let att = msgs[0]["attachments"].as_array().unwrap();
        assert_eq!(att.len(), 1);
        assert_eq!(att[0]["kind"], "audio");
        assert_eq!(msgs[1]["content"], "ok", "les autres messages intacts");
    }

    #[test]
    fn seules_les_dernieres_images_partent() {
        // Le cas du pilotage: une capture par etape. Les anciennes sont
        // remplacees par la mention, pas effacees en silence, sinon le modele
        // lit une suite d'etapes ou il n'a jamais rien regarde.
        let mut msgs: Vec<serde_json::Value> = (0..5)
            .map(|i| json!({"role":"user","content":format!("etape {i}"),"images":[format!("IMG{i}")]}))
            .collect();
        assert_eq!(borner_images(&mut msgs, 2), 3);
        for (i, m) in msgs.iter().enumerate() {
            if i < 3 {
                assert!(m.get("images").is_none(), "etape {i} devait etre allegee");
                assert!(m["content"].as_str().unwrap().contains("not re-sent"));
            } else {
                assert_eq!(m["images"][0], format!("IMG{i}"), "etape {i} gardee");
            }
        }
    }

    #[test]
    fn sans_image_rien_ne_bouge() {
        let mut msgs = vec![json!({"role":"user","content":"bonjour"})];
        assert_eq!(retirer_images(&mut msgs), 0);
        assert_eq!(msgs[0]["content"], "bonjour");
    }
}
