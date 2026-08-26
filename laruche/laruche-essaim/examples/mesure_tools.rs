//! Measures what the native tool block actually weighs in a request body.
//!
//! `reduire_sous_budget` trims tools FIRST, down to a floor of 4. This example
//! exists to check whether that order pays: if the tool block is small next to
//! the messages, cutting it costs the agent most of its capability and saves
//! almost nothing.
//!
//! Run: cargo run -p laruche-essaim --example mesure_tools

fn main() {
    let registry = laruche_essaim::abeille::AbeilleRegistry::new();
    laruche_essaim::abeilles::enregistrer_abeilles_builtin(&registry);

    // `schema_complet` returns the array directly, not a `{ "tools": [...] }` wrapper.
    let liste = registry
        .schema_complet()
        .as_array()
        .cloned()
        .unwrap_or_default();

    let n = liste.len();
    let total = serde_json::to_string(&liste).map(|s| s.len()).unwrap_or(0);
    println!("outils natifs        : {n}");
    println!("bloc tools serialise : {total} octets");
    println!("moyenne par outil    : {} octets", total / n.max(1));
    println!();
    println!("ce que le rabotage economise reellement:");
    for gardes in [16usize, 12, 8, 4] {
        let g = gardes.min(n);
        let sous = serde_json::to_string(&liste[..g]).map(|s| s.len()).unwrap_or(0);
        println!(
            "  garder {g:2} outils -> {sous:6} o  (economie {:5} o, soit {:.1}% de la garde de 76800)",
            total - sous,
            100.0 * (total - sous) as f64 / 76_800.0
        );
    }
}
