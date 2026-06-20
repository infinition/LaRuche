use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialEntry {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub cooldown_until: Option<i64>,
    #[serde(default)]
    pub invalid: bool,
    #[serde(default)]
    pub request_count: u64,
    #[serde(default)]
    pub label: Option<String>,
}

impl CredentialEntry {
    pub fn new(
        provider: impl Into<String>,
        api_key: impl Into<String>,
        base_url: Option<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            api_key: api_key.into(),
            base_url,
            cooldown_until: None,
            invalid: false,
            request_count: 0,
            label: None,
        }
    }

    pub fn disponible(&self, provider: &str, now: i64) -> bool {
        self.provider.eq_ignore_ascii_case(provider)
            && !self.invalid
            && self
                .cooldown_until
                .map(|until| until <= now)
                .unwrap_or(true)
            && !self.api_key.trim().is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialPool {
    pub entries: Vec<CredentialEntry>,
}

impl CredentialPool {
    pub fn new(entries: Vec<CredentialEntry>) -> Self {
        Self { entries }
    }

    pub fn prochain_disponible(&self, provider: &str, now: i64) -> Option<&CredentialEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.disponible(provider, now))
            .min_by_key(|entry| entry.request_count)
    }

    pub fn prochain_disponible_mut(
        &mut self,
        provider: &str,
        now: i64,
    ) -> Option<&mut CredentialEntry> {
        let index = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.disponible(provider, now))
            .min_by_key(|(_, entry)| entry.request_count)
            .map(|(index, _)| index)?;
        Some(&mut self.entries[index])
    }

    pub fn enregistrer_utilisation(&mut self, provider: &str, api_key: &str) -> bool {
        if let Some(entry) = self.trouver_mut(provider, api_key) {
            entry.request_count = entry.request_count.saturating_add(1);
            return true;
        }
        false
    }

    pub fn marquer_rate_limited(
        &mut self,
        provider: &str,
        api_key: &str,
        reset_at: Option<i64>,
        now: i64,
    ) -> bool {
        const DEFAULT_COOLDOWN_SECONDS: i64 = 60 * 60;
        if let Some(entry) = self.trouver_mut(provider, api_key) {
            entry.cooldown_until = Some(reset_at.unwrap_or(now + DEFAULT_COOLDOWN_SECONDS));
            return true;
        }
        false
    }

    pub fn marquer_invalide(&mut self, provider: &str, api_key: &str) -> bool {
        if let Some(entry) = self.trouver_mut(provider, api_key) {
            entry.invalid = true;
            return true;
        }
        false
    }

    pub fn reactiver_expirees(&mut self, now: i64) -> usize {
        let mut count = 0;
        for entry in &mut self.entries {
            if entry
                .cooldown_until
                .map(|until| until <= now)
                .unwrap_or(false)
            {
                entry.cooldown_until = None;
                count += 1;
            }
        }
        count
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    fn trouver_mut(&mut self, provider: &str, api_key: &str) -> Option<&mut CredentialEntry> {
        self.entries
            .iter_mut()
            .find(|entry| entry.provider.eq_ignore_ascii_case(provider) && entry.api_key == api_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(provider: &str, key: &str, count: u64) -> CredentialEntry {
        let mut entry = CredentialEntry::new(provider, key, None);
        entry.request_count = count;
        entry
    }

    #[test]
    fn selectionne_le_moins_utilise_disponible() {
        let pool = CredentialPool::new(vec![
            entry("openai", "key-a", 10),
            entry("openai", "key-b", 1),
        ]);

        let selected = pool.prochain_disponible("openai", 100).unwrap();

        assert_eq!(selected.api_key, "key-b");
    }

    #[test]
    fn saute_cooldown_et_invalides() {
        let mut cooling = entry("openai", "cooling", 0);
        cooling.cooldown_until = Some(200);
        let mut invalid = entry("openai", "invalid", 0);
        invalid.invalid = true;
        let pool = CredentialPool::new(vec![cooling, invalid, entry("openai", "ok", 5)]);

        assert_eq!(
            pool.prochain_disponible("openai", 100).unwrap().api_key,
            "ok"
        );
    }

    #[test]
    fn rate_limit_pose_un_cooldown_puis_reactive() {
        let mut pool = CredentialPool::new(vec![entry("anthropic", "key", 0)]);

        assert!(pool.marquer_rate_limited("anthropic", "key", Some(120), 10));
        assert!(pool.prochain_disponible("anthropic", 100).is_none());
        assert_eq!(pool.reactiver_expirees(121), 1);
        assert!(pool.prochain_disponible("anthropic", 121).is_some());
    }

    #[test]
    fn serialization_json_preserve_etat() {
        let mut pool = CredentialPool::new(vec![entry("codex", "token", 2)]);
        pool.marquer_invalide("codex", "token");

        let roundtrip = CredentialPool::from_json(&pool.to_json().unwrap()).unwrap();

        assert_eq!(roundtrip, pool);
    }
}
