//! L'arbre d'accessibilite, c'est-a-dire le `read` du bureau.
//!
//! Un modele sans vision ne peut rien faire d'une capture d'ecran. Et meme un
//! modele qui voit vise mal: il lit une position dans une image reduite, se
//! trompe de quelques pixels, et clique a cote d'un bouton de 24 pixels de
//! haut. C'est le meme probleme que le navigateur a resolu il y a longtemps
//! avec ses `ref_N`, et la reponse est la meme ici.
//!
//! Windows expose UI Automation: la hierarchie des controles d'une fenetre,
//! avec leur nom, leur type, leur etat et leur rectangle. On la parcourt, on
//! numerote ce qui est actionnable, et le modele agit sur un numero. Trois
//! consequences qui comptent:
//!
//!   - plus de coordonnees a deviner, donc plus de clic a cote;
//!   - `Invoke` et `SetValue` sont deterministes: ils appellent le controle,
//!     ils ne simulent pas un humain qui vise. Ils marchent meme quand la
//!     fenetre n'est pas au premier plan;
//!   - un modele sans vision travaille normalement, en lisant du texte.
//!
//! La souris reste la solution de repli quand un controle n'expose aucun motif
//! exploitable, ce qui arrive avec les interfaces dessinees a la main (jeux,
//! Electron mal balise, canvas).

use anyhow::{anyhow, Result};
use std::sync::Mutex;
use uiautomation::controls::ControlType;
use uiautomation::core::UICacheRequest;
use uiautomation::patterns::{
    UIExpandCollapsePattern, UIInvokePattern, UILegacyIAccessiblePattern, UIRangeValuePattern,
    UIScrollItemPattern, UISelectionItemPattern, UITogglePattern, UIValuePattern,
};
use uiautomation::types::{TreeScope, UIProperty};
use uiautomation::{UIAutomation, UIElement};

/// Plafonds du parcours. Une fenetre de navigateur expose des milliers de
/// noeuds; les enumerer tous couterait des secondes et noierait le modele.
const MAX_ELEMENTS: usize = 300;
const MAX_PROFONDEUR: usize = 30;
const MAX_TEXTES: usize = 120;

/// Ce qu'on retient d'un element entre deux appels.
///
/// Pas l'`UIElement` lui-meme: c'est un objet COM, lie au thread qui l'a cree,
/// et le stocker entre deux appels d'outil serait un bug de threading en
/// attente. On garde de quoi le RETROUVER, plus son rectangle, qui sert de
/// repli quand il a disparu.
#[derive(Clone, Debug, PartialEq)]
pub struct Cible {
    /// Identifiant stable quand l'application en fournit un. Beaucoup n'en
    /// donnent pas, d'ou les trois champs suivants qui completent l'identite.
    pub automation_id: String,
    pub nom: String,
    pub genre: String,
    /// Rectangle physique, pour le repli a la souris et pour departager deux
    /// controles homonymes.
    pub rect: (i32, i32, i32, i32),
}

impl Cible {
    pub fn centre(&self) -> (f64, f64) {
        let (l, t, r, b) = self.rect;
        ((l + r) as f64 / 2.0, (t + b) as f64 / 2.0)
    }
}

static REFS: Mutex<Vec<Cible>> = Mutex::new(Vec::new());

pub fn cible(numero: usize) -> Option<Cible> {
    REFS.lock().ok()?.get(numero.checked_sub(1)?).cloned()
}

pub fn oublier_refs() {
    if let Ok(mut r) = REFS.lock() {
        r.clear();
    }
}

fn nouvelle_automation() -> Result<UIAutomation> {
    // Creee a chaque appel: COM est initialise par thread, et le travail
    // bloquant de cet outil peut atterrir sur n'importe quel thread du pool.
    UIAutomation::new().map_err(|e| anyhow!("UI Automation unavailable: {e}"))
}

/// La requete de cache: LA raison pour laquelle cette lecture est rapide.
///
/// Sans elle, chaque propriete lue est un aller-retour COM vers le processus
/// proprietaire de la fenetre. Mesure sur une fenetre Electron reelle: 36
/// secondes pour 160 controles, ce qui est inutilisable dans une boucle
/// d'agent. Avec elle, tout l'arbre et toutes ses proprietes arrivent en un
/// seul appel.
fn cache(auto: &UIAutomation) -> Result<UICacheRequest> {
    let c = auto
        .create_cache_request()
        .map_err(|e| anyhow!("cannot prepare the UIA cache: {e}"))?;
    for p in [
        UIProperty::Name,
        UIProperty::ControlType,
        UIProperty::BoundingRectangle,
        UIProperty::IsEnabled,
        UIProperty::IsOffscreen,
        UIProperty::AutomationId,
    ] {
        c.add_property(p)
            .map_err(|e| anyhow!("cannot cache {p:?}: {e}"))?;
    }
    Ok(c)
}

/// Tous les controles d'une fenetre, en UN aller-retour.
fn descendants(auto: &UIAutomation, fenetre: &UIElement) -> Result<Vec<UIElement>> {
    let condition = auto
        .get_control_view_condition()
        .map_err(|e| anyhow!("cannot build the control filter: {e}"))?;
    let cache = cache(auto)?;
    fenetre
        .find_all_build_cache(TreeScope::Subtree, &condition, &cache)
        .map_err(|e| anyhow!("cannot read the window tree: {e}"))
}

/// Le type de controle, en un mot lisible par un modele.
fn genre(t: ControlType) -> &'static str {
    match t {
        ControlType::Button => "button",
        ControlType::CheckBox => "checkbox",
        ControlType::ComboBox => "combobox",
        ControlType::Edit => "input",
        ControlType::Hyperlink => "link",
        ControlType::ListItem => "item",
        ControlType::List => "list",
        ControlType::MenuItem => "menuitem",
        ControlType::RadioButton => "radio",
        ControlType::Slider => "slider",
        ControlType::Tab => "tabs",
        ControlType::TabItem => "tab",
        ControlType::Text => "text",
        ControlType::Tree => "tree",
        ControlType::TreeItem => "treeitem",
        ControlType::Window => "window",
        ControlType::Document => "document",
        ControlType::Group => "group",
        ControlType::MenuBar => "menubar",
        ControlType::ToolBar => "toolbar",
        ControlType::Image => "image",
        ControlType::Table => "table",
        ControlType::DataItem => "row",
        ControlType::SplitButton => "splitbutton",
        ControlType::Spinner => "spinner",
        ControlType::ProgressBar => "progress",
        _ => "control",
    }
}

/// Ce type de controle merite-t-il un numero, c'est-a-dire peut-on agir dessus?
///
/// `Document` en fait partie, et ce n'est pas evident: c'est le type que porte
/// la zone de saisie du Bloc-notes, celle de la plupart des editeurs, et le
/// corps d'une page dans un navigateur. Sans lui, l'arbre listait fidelement
/// tous les boutons d'une fenetre et pas l'endroit ou l'on ecrit.
fn actionnable(t: ControlType) -> bool {
    matches!(
        t,
        ControlType::Document
            | ControlType::Button
            | ControlType::CheckBox
            | ControlType::ComboBox
            | ControlType::Edit
            | ControlType::Hyperlink
            | ControlType::ListItem
            | ControlType::MenuItem
            | ControlType::RadioButton
            | ControlType::Slider
            | ControlType::TabItem
            | ControlType::TreeItem
            | ControlType::SplitButton
            | ControlType::Spinner
            | ControlType::DataItem
    )
}

/// La fenetre a lire: celle qui est nommee, sinon celle qui a le focus.
fn fenetre_cible(auto: &UIAutomation, filtre: Option<&str>) -> Result<UIElement> {
    let racine = auto
        .get_root_element()
        .map_err(|e| anyhow!("cannot reach the desktop root: {e}"))?;
    let walker = auto
        .get_control_view_walker()
        .map_err(|e| anyhow!("cannot walk the tree: {e}"))?;

    if let Some(f) = filtre {
        let besoin = f.to_lowercase();
        let mut enfant = walker.get_first_child(&racine).ok();
        let mut vues = Vec::new();
        while let Some(e) = enfant {
            let nom = e.get_name().unwrap_or_default();
            if !nom.trim().is_empty() {
                if nom.to_lowercase().contains(&besoin) {
                    return Ok(e);
                }
                vues.push(nom);
            }
            enfant = walker.get_next_sibling(&e).ok();
        }
        return Err(anyhow!(
            "No top-level window matching \"{f}\". Open ones: {}",
            if vues.is_empty() {
                "none visible".to_string()
            } else {
                vues.join(" | ")
            }
        ));
    }

    // Sans filtre: on part de l'element qui a le focus et on remonte jusqu'a
    // l'enfant direct du bureau, qui est la fenetre de premier plan.
    let focus = auto
        .get_focused_element()
        .map_err(|e| anyhow!("nothing has focus: {e}"))?;
    let mut courant = focus;
    for _ in 0..MAX_PROFONDEUR {
        let parent = match walker.get_parent(&courant) {
            Ok(p) => p,
            Err(_) => return Ok(courant),
        };
        if auto.compare_elements(&parent, &racine).unwrap_or(false) {
            return Ok(courant);
        }
        courant = parent;
    }
    Ok(courant)
}

/// Le resultat d'une lecture, deja mis en forme pour le modele.
pub struct Lecture {
    pub titre: String,
    pub lignes: Vec<String>,
    pub textes: Vec<String>,
    pub tronque: bool,
}

/// Parcourt la fenetre et numerote ce qui est actionnable.
pub fn lire(filtre: Option<&str>) -> Result<Lecture> {
    let auto = nouvelle_automation()?;
    let fenetre = fenetre_cible(&auto, filtre)?;
    let titre = fenetre.get_name().unwrap_or_default();
    let tous = descendants(&auto, &fenetre)?;
    let tronque = tous.len() > MAX_ELEMENTS * 4;

    let mut cibles: Vec<Cible> = Vec::new();
    let mut lignes = Vec::new();
    let mut textes = Vec::new();
    let mut plein = false;

    for element in tous {
        if element.is_cached_offscreen().unwrap_or(false) {
            continue;
        }
        let Ok(control) = element.get_cached_control_type() else {
            continue;
        };
        let nom = element.get_cached_name().unwrap_or_default();
        let nom = nom.split_whitespace().collect::<Vec<_>>().join(" ");

        if !actionnable(control) {
            // Le texte statique n'est pas actionnable mais il porte tout le sens
            // de la fenetre: sans lui un modele sans vision agit a l'aveugle.
            if matches!(control, ControlType::Text)
                && !nom.trim().is_empty()
                && textes.len() < MAX_TEXTES
                && !textes.contains(&nom)
            {
                textes.push(nom);
            }
            continue;
        }
        if plein {
            continue;
        }

        let rect = element
            .get_cached_bounding_rectangle()
            .map(|r| (r.get_left(), r.get_top(), r.get_right(), r.get_bottom()))
            .unwrap_or((0, 0, 0, 0));
        // Un controle de surface nulle est present dans l'arbre mais invisible
        // et injoignable: le numeroter n'aurait servi qu'a le faire viser.
        if rect.2 - rect.0 < 1 || rect.3 - rect.1 < 1 {
            continue;
        }

        // Les seules lectures qui restent hors cache, et elles ne concernent
        // qu'une poignee de controles: la valeur d'un champ et l'etat d'une
        // case. Les mettre en cache couterait un motif par element, donc plus
        // cher que de les demander pour les rares qui en ont un.
        // Un Document porte son contenu dans son motif Value comme un champ:
        // c'est ce qui permet de RELIRE ce qu'on vient d'ecrire.
        let valeur = matches!(
            control,
            ControlType::Edit | ControlType::ComboBox | ControlType::Document
        )
        .then(|| {
            element
                .get_pattern::<UIValuePattern>()
                .ok()
                .and_then(|p| p.get_value().ok())
                .filter(|v| !v.trim().is_empty())
        })
        .flatten();
        let coche = matches!(control, ControlType::CheckBox | ControlType::RadioButton)
            .then(|| {
                element
                    .get_pattern::<UITogglePattern>()
                    .ok()
                    .and_then(|p| p.get_toggle_state().ok())
                    .map(|s| format!("{s:?}").to_lowercase())
            })
            .flatten();
        let actif = element.is_cached_enabled().unwrap_or(true);

        let numero = cibles.len() + 1;
        let mut ligne = format!("ref_{numero} <{}>", genre(control));
        if !actif {
            ligne.push_str(" [disabled]");
        }
        if let Some(c) = &coche {
            ligne.push_str(&format!(" [{c}]"));
        }
        if !nom.trim().is_empty() {
            ligne.push(' ');
            ligne.push_str(nom.chars().take(80).collect::<String>().trim());
        }
        if let Some(v) = &valeur {
            ligne.push_str(&format!(" = {}", v.chars().take(60).collect::<String>()));
        }
        lignes.push(ligne);
        cibles.push(Cible {
            automation_id: element.get_cached_automation_id().unwrap_or_default(),
            nom,
            genre: genre(control).to_string(),
            rect,
        });
        if cibles.len() >= MAX_ELEMENTS {
            plein = true;
        }
    }

    *REFS.lock().unwrap() = cibles;
    Ok(Lecture {
        titre,
        lignes,
        textes,
        tronque: tronque || plein,
    })
}

/// Les elements dont le libelle contient `quoi`, sans casse.
///
/// `lire` peut rendre trois cents lignes plus cent vingt lignes de texte, et
/// tronque en silence au-dela. Sur une grosse fenetre Electron ou une suite
/// bureautique, c'est une facture de jetons a chaque lecture pour trouver un
/// bouton dont on connait deja le nom. Le navigateur avait `find` depuis
/// longtemps; le bureau non, sans autre raison que l'oubli.
///
/// Les numeros rendus sont ceux de la lecture qui vient d'avoir lieu, donc
/// utilisables directement: la recherche EST une lecture, elle renumerote comme
/// n'importe quelle autre.
pub fn chercher(filtre: Option<&str>, quoi: &str) -> Result<(String, Vec<String>, bool)> {
    let l = lire(filtre)?;
    let besoin = quoi.to_lowercase();
    let trouves: Vec<String> = l
        .lignes
        .iter()
        .filter(|ligne| ligne.to_lowercase().contains(&besoin))
        .cloned()
        .collect();
    Ok((l.titre, trouves, l.tronque))
}

/// Retrouve un element a partir de ce qu'on a memorise de lui.
///
/// Le rechercher plutot que le garder est le prix a payer pour ne pas stocker
/// un objet COM entre deux appels d'outil: COM est lie au thread, et le travail
/// bloquant atterrit sur n'importe quel thread du pool. L'identite est un
/// faisceau plutot qu'une cle, parce que beaucoup d'applications ne donnent
/// aucun `AutomationId`: on exige le meme type et le meme rectangle, et le nom
/// ou l'identifiant en plus.
fn retrouver(auto: &UIAutomation, cible: &Cible, filtre: Option<&str>) -> Option<UIElement> {
    let fenetre = fenetre_cible(auto, filtre).ok()?;
    let tous = descendants(auto, &fenetre).ok()?;
    let mut repli = None;
    for element in tous {
        let Ok(control) = element.get_cached_control_type() else {
            continue;
        };
        if genre(control) != cible.genre {
            continue;
        }
        let nom = element.get_cached_name().unwrap_or_default();
        let nom = nom.split_whitespace().collect::<Vec<_>>().join(" ");
        let aid = element.get_cached_automation_id().unwrap_or_default();
        let rect = element
            .get_cached_bounding_rectangle()
            .map(|r| (r.get_left(), r.get_top(), r.get_right(), r.get_bottom()))
            .unwrap_or((0, 0, 0, 0));

        if !cible.automation_id.is_empty() && aid == cible.automation_id {
            return Some(element);
        }
        if nom == cible.nom && rect == cible.rect {
            return Some(element);
        }
        // Le meme nom a un autre endroit reste un candidat: une fenetre qui a
        // simplement bouge ne doit pas rendre ses refs inutilisables.
        if repli.is_none() && nom == cible.nom && !nom.is_empty() {
            repli = Some(element);
        }
    }
    repli
}

/// Ce qui s'est reellement passe, pour le dire au modele sans broder.
pub enum Effet {
    /// Le controle a ete appele par son motif d'automatisation.
    Motif(&'static str),
    /// Aucun motif exploitable: l'appelant doit cliquer physiquement.
    ClicRequis,
    /// Le controle porte une valeur sur une plage: un clic dessus n'a pas de
    /// sens, il faut dire laquelle. Porte le minimum, le maximum et la valeur
    /// courante pour que le refus soit utilisable tel quel.
    ///
    /// Sans ce cas, un curseur tombait dans le repli physique et se faisait
    /// cliquer en son centre, donc regle au milieu de sa course. Une action
    /// fausse rapportee comme un succes est pire qu'une erreur.
    PlageRequise(f64, f64, f64),
}

/// Actionne un element numerote, par son motif quand il en expose un.
pub fn actionner(numero: usize) -> Result<(Cible, Effet)> {
    let c = cible(numero).ok_or_else(|| {
        anyhow!("No element ref_{numero}. Run read again: refs are renumbered by every read.")
    })?;
    let auto = nouvelle_automation()?;
    let Some(element) = retrouver(&auto, &c, None) else {
        // Disparu de l'arbre: la fenetre a change. Le rectangle memorise reste
        // exploitable pour un clic, mais c'est un pari, et il faut le dire.
        return Ok((c, Effet::ClicRequis));
    };
    let _ = element.set_focus();
    // Une valeur sur une plage se REGLE, elle ne se clique pas. Ce test passe
    // avant tous les motifs d'action: un curseur expose parfois Invoke, et
    // l'invoquer ne veut rien dire.
    if let Ok(p) = element.get_pattern::<UIRangeValuePattern>() {
        if !p.is_readonly().unwrap_or(true) {
            return Ok((
                c,
                Effet::PlageRequise(
                    p.get_minimum().unwrap_or(0.0),
                    p.get_maximum().unwrap_or(0.0),
                    p.get_value().unwrap_or(0.0),
                ),
            ));
        }
    }
    if let Ok(p) = element.get_pattern::<UIInvokePattern>() {
        if p.invoke().is_ok() {
            return Ok((c, Effet::Motif("invoke")));
        }
    }
    if let Ok(p) = element.get_pattern::<UITogglePattern>() {
        if p.toggle().is_ok() {
            return Ok((c, Effet::Motif("toggle")));
        }
    }
    if let Ok(p) = element.get_pattern::<UISelectionItemPattern>() {
        if p.select().is_ok() {
            return Ok((c, Effet::Motif("select")));
        }
    }
    // Deplier une liste deroulante ou un noeud d'arbre. Beaucoup de combos
    // n'exposent pas Invoke, et sans ce motif le seul recours etait un clic
    // aveugle sur la fleche.
    if let Ok(p) = element.get_pattern::<UIExpandCollapsePattern>() {
        use uiautomation::types::ExpandCollapseState;
        let ouvert = matches!(p.get_state(), Ok(ExpandCollapseState::Expanded));
        let fait = if ouvert { p.collapse() } else { p.expand() };
        if fait.is_ok() {
            return Ok((c, Effet::Motif(if ouvert { "collapse" } else { "expand" })));
        }
    }
    // Dernier recours avant de viser a la souris: l'action par defaut de la
    // vieille interface MSAA, que beaucoup de controles Win32 anciens sont
    // seuls a exposer.
    if let Ok(p) = element.get_pattern::<UILegacyIAccessiblePattern>() {
        if p.do_default_action().is_ok() {
            return Ok((c, Effet::Motif("default action")));
        }
    }
    Ok((c, Effet::ClicRequis))
}

/// Amene un element numerote dans la partie visible de son conteneur.
///
/// C'est le pendant vision-libre du `scroll` a la molette. `lire` saute tout
/// element hors ecran, donc ce qui est sous la ligne de flottaison n'existe
/// pas pour le modele; sans ce chemin, le seul moyen de l'atteindre etait une
/// capture puis une molette en pixels, c'est-a-dire sortir du chemin sans
/// vision pour une raison purement mecanique.
pub fn defiler_vers(numero: usize) -> Result<(Cible, Effet)> {
    let c = cible(numero).ok_or_else(|| {
        anyhow!("No element ref_{numero}. Run read again: refs are renumbered by every read.")
    })?;
    let auto = nouvelle_automation()?;
    let Some(element) = retrouver(&auto, &c, None) else {
        return Ok((c, Effet::ClicRequis));
    };
    if let Ok(p) = element.get_pattern::<UIScrollItemPattern>() {
        if p.scroll_into_view().is_ok() {
            return Ok((c, Effet::Motif("scroll into view")));
        }
    }
    Ok((c, Effet::ClicRequis))
}

/// Regle un controle qui porte une valeur sur une plage: curseur, compteur,
/// barre de progression modifiable.
///
/// Retourne la valeur reellement appliquee, que le controle ajuste souvent a
/// son pas: demander 7 sur un curseur qui avance de cinq en cinq en pose 5, et
/// le taire ferait croire a un reglage qui n'a pas eu lieu.
pub fn regler_plage(numero: usize, valeur: f64) -> Result<(Cible, f64)> {
    let c = cible(numero).ok_or_else(|| anyhow!("No element ref_{numero}. Run read again."))?;
    let auto = nouvelle_automation()?;
    let element = retrouver(&auto, &c, None)
        .ok_or_else(|| anyhow!("ref_{numero} is gone from the tree: the window changed."))?;
    let p = element.get_pattern::<UIRangeValuePattern>().map_err(|_| {
        anyhow!(
            "ref_{numero} <{}> holds no range: it is not a slider or a spinner. Use click or fill.",
            c.genre
        )
    })?;
    if p.is_readonly().unwrap_or(false) {
        return Err(anyhow!("ref_{numero} <{}> is read only.", c.genre));
    }
    let (min, max) = (
        p.get_minimum().unwrap_or(0.0),
        p.get_maximum().unwrap_or(0.0),
    );
    // Borner plutot que laisser le motif echouer: hors plage, `set_value` rend
    // une erreur COM opaque que personne ne sait lire.
    let borne = valeur.clamp(min.min(max), max.max(min));
    let _ = element.set_focus();
    p.set_value(borne)
        .map_err(|e| anyhow!("cannot set ref_{numero}: {e}"))?;
    Ok((c, p.get_value().unwrap_or(borne)))
}

/// Ecrit dans un champ numerote.
pub fn remplir(numero: usize, valeur: &str) -> Result<(Cible, Effet)> {
    let c = cible(numero).ok_or_else(|| {
        anyhow!("No element ref_{numero}. Run read again: refs are renumbered by every read.")
    })?;
    let auto = nouvelle_automation()?;
    let Some(element) = retrouver(&auto, &c, None) else {
        return Ok((c, Effet::ClicRequis));
    };
    let _ = element.set_focus();
    // Un curseur ou un compteur se remplit avec un nombre, par son motif de
    // plage. Avant, `fill` sur un curseur ne trouvait pas de motif Value et
    // retombait sur "cliquer puis taper", ce qui ne regle rien.
    if let Ok(nombre) = valeur.trim().parse::<f64>() {
        if let Ok(p) = element.get_pattern::<UIRangeValuePattern>() {
            if !p.is_readonly().unwrap_or(true) {
                let (min, max) = (
                    p.get_minimum().unwrap_or(0.0),
                    p.get_maximum().unwrap_or(0.0),
                );
                if p.set_value(nombre.clamp(min.min(max), max.max(min)))
                    .is_ok()
                {
                    return Ok((c, Effet::Motif("rangevalue")));
                }
            }
        }
    }
    if let Ok(p) = element.get_pattern::<UIValuePattern>() {
        if p.set_value(valeur).is_ok() {
            return Ok((c, Effet::Motif("setvalue")));
        }
    }
    // Pas de motif Value: le champ ne se remplit qu'a la frappe. L'appelant
    // clique dedans puis tape, ce qui marche sur les interfaces dessinees.
    Ok((c, Effet::ClicRequis))
}

/// Donne le focus a un element numerote, sans l'actionner.
pub fn focaliser(numero: usize) -> Result<Cible> {
    let c = cible(numero).ok_or_else(|| anyhow!("No element ref_{numero}. Run read again."))?;
    let auto = nouvelle_automation()?;
    match retrouver(&auto, &c, None) {
        Some(e) => {
            e.set_focus()
                .map_err(|err| anyhow!("cannot focus ref_{numero}: {err}"))?;
            Ok(c)
        }
        None => Err(anyhow!(
            "ref_{numero} is gone from the tree: the window changed. Run read again."
        )),
    }
}

/// Met une fenetre au premier plan, par un fragment de son titre.
pub fn activer_fenetre(filtre: &str) -> Result<String> {
    let auto = nouvelle_automation()?;
    let fenetre = fenetre_cible(&auto, Some(filtre))?;
    let titre = fenetre.get_name().unwrap_or_default();
    // set_focus sur la fenetre elle-meme fait le travail dans la majorite des
    // cas; SetForegroundWindow rattrape ce que UIA ne peut pas, notamment une
    // fenetre reduite.
    let handle = fenetre.get_native_window_handle().ok();
    let _ = fenetre.set_focus();
    if let Some(h) = handle {
        unsafe {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{
                SetForegroundWindow, ShowWindow, SW_RESTORE,
            };
            let hwnd = HWND(Into::<isize>::into(h) as *mut core::ffi::c_void);
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
    }
    // Les refs appartenaient a l'ancienne fenetre: les garder inviterait a
    // cliquer sur un numero qui ne designe plus rien.
    oublier_refs();
    Ok(titre)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_genres_couvrent_ce_qui_est_actionnable() {
        // Tout ce qui est actionnable doit avoir un nom lisible, sinon le modele
        // voit `<control>` et ne sait pas ce qu'il manipule.
        for t in [
            ControlType::Button,
            ControlType::CheckBox,
            ControlType::ComboBox,
            ControlType::Edit,
            ControlType::Hyperlink,
            ControlType::MenuItem,
            ControlType::RadioButton,
            ControlType::TabItem,
            ControlType::TreeItem,
        ] {
            assert!(actionnable(t), "{t:?} devrait etre actionnable");
            assert_ne!(genre(t), "control", "{t:?} n'a pas de nom lisible");
        }
        // Et le texte statique ne doit PAS recevoir de numero: il n'y a rien a
        // y faire, et le numeroter noierait les vrais controles.
        assert!(!actionnable(ControlType::Text));
    }

    #[test]
    fn le_centre_dun_rectangle_est_son_centre() {
        let c = Cible {
            automation_id: String::new(),
            nom: String::new(),
            genre: String::new(),
            rect: (100, 200, 300, 240),
        };
        assert_eq!(c.centre(), (200.0, 220.0));
    }

    /// Lecture reelle de la fenetre au premier plan. Ignore par defaut: il faut
    /// une session graphique, et le resultat depend de ce qui est ouvert.
    ///
    ///   cargo test -p laruche-essaim --lib arbre_vivant -- --ignored --nocapture
    #[test]
    #[ignore = "requires a real desktop session"]
    fn arbre_vivant() {
        let lecture = lire(None).expect("lecture de la fenetre au premier plan");
        println!("Fenetre: {}", lecture.titre);
        println!("{} element(s) actionnable(s):", lecture.lignes.len());
        for l in lecture.lignes.iter().take(25) {
            println!("  {l}");
        }
        println!("{} bloc(s) de texte", lecture.textes.len());
        assert!(
            !lecture.lignes.is_empty() || !lecture.textes.is_empty(),
            "une fenetre reelle expose forcement quelque chose"
        );
    }
}
