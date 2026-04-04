//! Built-in Abeilles (tools) for the Essaim agent.

pub mod browser;
pub mod calendrier;
pub mod knowledge;
// Re-export plugin loader
pub mod plugins;
pub use plugins::charger_plugins;
pub mod delegation;
pub mod essaim_status;
pub mod fichiers;
pub mod math;
pub mod recherche_fichiers;
pub mod shell;
pub mod web_fetch;
pub mod web_recherche;

use crate::abeille::AbeilleRegistry;

/// Register all built-in Abeilles into the registry.
pub fn enregistrer_abeilles_builtin(registry: &mut AbeilleRegistry) {
    // File operations
    registry.enregistrer(Box::new(fichiers::FileRead));
    registry.enregistrer(Box::new(fichiers::FileList));
    registry.enregistrer(Box::new(fichiers::FileWrite));
    registry.enregistrer(Box::new(recherche_fichiers::FileSearch));
    // Shell
    registry.enregistrer(Box::new(shell::ShellExec));
    // Web
    registry.enregistrer(Box::new(web_recherche::WebSearch));
    registry.enregistrer(Box::new(web_fetch::WebFetch));
    // Math
    registry.enregistrer(Box::new(math::MathEval));
    // Calendar
    registry.enregistrer(Box::new(calendrier::CalendarAdd));
    registry.enregistrer(Box::new(calendrier::CalendarList));
    // Browser
    registry.enregistrer(Box::new(browser::BrowserNavigate));
    registry.enregistrer(Box::new(browser::BrowserScreenshot));
    // System
    registry.enregistrer(Box::new(essaim_status::SystemInfo));

    tracing::info!(count = registry.noms().len(), "Built-in Abeilles registered");
}

/// Register the delegate abeille (requires registry reference + config).
/// Call this AFTER enregistrer_abeilles_builtin, passing an Arc<AbeilleRegistry>.
pub fn enregistrer_delegation(
    registry: &mut AbeilleRegistry,
    sub_registry: std::sync::Arc<AbeilleRegistry>,
    config: crate::brain::EssaimConfig,
) {
    registry.enregistrer(Box::new(delegation::Delegate {
        registry: sub_registry,
        config,
    }));
    tracing::info!("Delegate abeille registered (sub-agent capable)");
}
