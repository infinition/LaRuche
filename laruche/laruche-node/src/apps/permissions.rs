use super::AppManifest;
use serde::Serialize;
use std::collections::BTreeSet;

pub(crate) const CATALOG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PermissionDescriptor {
    id: &'static str,
    risk: &'static str,
    title_fr: &'static str,
    title_en: &'static str,
}

#[derive(Debug)]
pub(crate) enum GrantError {
    RequiredUnavailable(String),
    MissingRequired(String),
    Undeclared(String),
    Unavailable(String),
}

pub(crate) fn catalog() -> Vec<PermissionDescriptor> {
    vec![
        PermissionDescriptor {
            id: "agents.invoke",
            risk: "high",
            title_fr: "Solliciter les agents autorisés (consomme des tokens)",
            title_en: "Invoke authorized agents (uses model tokens)",
        },
        PermissionDescriptor {
            id: super::model::NETWORK_PERMISSION,
            risk: "high",
            title_fr: "Joindre les hotes declares par le paquet",
            title_en: "Reach the hosts declared by the package",
        },
        PermissionDescriptor {
            id: "laruche.files",
            risk: "high",
            title_fr: "Lire et ecrire des fichiers dans son dossier",
            title_en: "Read and write files in its own folder",
        },
        PermissionDescriptor {
            id: "storage.private",
            risk: "low",
            title_fr: "Stockage privé de l'app",
            title_en: "Private app storage",
        },
        PermissionDescriptor {
            id: "ui.locale.read",
            risk: "low",
            title_fr: "Lire la langue de l'interface",
            title_en: "Read the interface language",
        },
        PermissionDescriptor {
            id: "ui.theme.read",
            risk: "low",
            title_fr: "Lire le thème de l'interface",
            title_en: "Read the interface theme",
        },
    ]
}

/// A capability the node actually implements.
///
/// The consent screen greys out anything absent here, which is the right
/// default: a checkbox that grants nothing is worse than no checkbox, because
/// the user believes they decided something. The corollary is that a name
/// belongs in this list the day its enforcement exists, and not a day earlier.
pub(crate) fn is_available(permission: &str) -> bool {
    matches!(
        permission,
        "storage.private"
            | "ui.locale.read"
            | "ui.theme.read"
            | "agents.invoke"
            | "laruche.files"
            | super::model::NETWORK_PERMISSION
    )
}

pub(crate) fn validate_grants(
    manifest: &AppManifest,
    requested: Vec<String>,
) -> Result<Vec<String>, GrantError> {
    for permission in &manifest.permissions.required {
        if !is_available(permission) {
            return Err(GrantError::RequiredUnavailable(permission.clone()));
        }
    }

    let declared: BTreeSet<&str> = manifest
        .permissions
        .required
        .iter()
        .chain(&manifest.permissions.optional)
        .map(String::as_str)
        .collect();
    let grants: BTreeSet<String> = requested.into_iter().collect();
    for permission in &grants {
        if !declared.contains(permission.as_str()) {
            return Err(GrantError::Undeclared(permission.clone()));
        }
        if !is_available(permission) {
            return Err(GrantError::Unavailable(permission.clone()));
        }
    }
    for permission in &manifest.permissions.required {
        if !grants.contains(permission) {
            return Err(GrantError::MissingRequired(permission.clone()));
        }
    }
    Ok(grants.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::AppManifest;

    /// Every catalogued capability must also be available, and the reverse.
    ///
    /// The consent screen greys out whatever `is_available` rejects, so a name
    /// present in one list and absent from the other produces a checkbox that
    /// either cannot be ticked or grants something the screen never described.
    /// network.fetch spent its first hours in exactly that state.
    #[test]
    fn le_catalogue_et_la_disponibilite_disent_la_meme_chose() {
        for descripteur in catalog() {
            assert!(
                is_available(descripteur.id),
                "{} est au catalogue mais indisponible",
                descripteur.id
            );
        }
        for connue in [
            "storage.private",
            "ui.locale.read",
            "ui.theme.read",
            "agents.invoke",
            "laruche.files",
            super::super::model::NETWORK_PERMISSION,
        ] {
            assert!(is_available(connue), "{connue} devrait etre disponible");
            assert!(
                catalog().iter().any(|d| d.id == connue),
                "{connue} est disponible mais absente du catalogue"
            );
        }
    }

    #[test]
    fn une_capacite_inconnue_reste_indisponible() {
        assert!(!is_available("files.write"));
        assert!(!is_available("network"));
        assert!(!is_available("network.fetch.all"));
    }

    fn manifest(required: &[&str], optional: &[&str]) -> AppManifest {
        let required = serde_json::to_string(required).unwrap();
        let optional = serde_json::to_string(optional).unwrap();
        AppManifest::parse_and_validate(&format!(
            r#"{{
              "apiVersion": 1,
              "id": "dev.laruche.test",
              "name": "Test",
              "version": "1.0.0",
              "description": "Permissions test",
              "publisher": {{"name": "LaRuche"}},
              "ui": {{"views": [{{"id": "main", "title": "Main", "entry": "ui/index.html"}}]}},
              "permissions": {{"required": {required}, "optional": {optional}}}
            }}"#
        ))
        .unwrap()
    }

    #[test]
    fn requires_every_required_permission() {
        let manifest = manifest(&["storage.private"], &[]);
        assert!(matches!(
            validate_grants(&manifest, Vec::new()),
            Err(GrantError::MissingRequired(permission)) if permission == "storage.private"
        ));
    }

    #[test]
    fn accepts_declared_available_optional_permissions() {
        let manifest = manifest(&["storage.private"], &["ui.theme.read"]);
        let grants = validate_grants(
            &manifest,
            vec!["ui.theme.read".into(), "storage.private".into()],
        )
        .unwrap();
        assert_eq!(grants, vec!["storage.private", "ui.theme.read"]);
    }

    #[test]
    fn refuses_unimplemented_required_permissions() {
        let manifest = manifest(&["notifications.send"], &[]);
        assert!(matches!(
            validate_grants(&manifest, vec!["notifications.send".into()]),
            Err(GrantError::RequiredUnavailable(permission)) if permission == "notifications.send"
        ));
    }
}
