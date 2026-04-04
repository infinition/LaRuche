//! Proof of Proximity authentication system.
//!
//! Instead of passwords, LaRuche uses physical proximity for authorization.
//! A new device must be approved by pressing a physical button on the
//! LaRuche box or tapping it with NFC. This generates a signed auth token
//! bound to a trust circle.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

/// Trust circles define access levels for authorized devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustCircle {
    /// Full access, all models, high priority
    Family,
    /// Professional access, configurable model access, normal priority
    Office,
    /// Limited access, restricted models, low priority
    Guest,
}

impl TrustCircle {
    pub fn max_qos(&self) -> crate::qos::QosLevel {
        match self {
            Self::Family => crate::qos::QosLevel::High,
            Self::Office => crate::qos::QosLevel::Normal,
            Self::Guest => crate::qos::QosLevel::Low,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Family => "Famille / Équipe",
            Self::Office => "Bureau",
            Self::Guest => "Invité",
        }
    }
}

/// Authentication token issued after physical approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// Unique token ID
    pub token_id: Uuid,

    /// The device this token was issued to
    pub device_id: Uuid,

    /// Human-friendly device name
    pub device_name: String,

    /// Trust circle this device belongs to
    pub circle: TrustCircle,

    /// When this token was issued
    pub issued_at: DateTime<Utc>,

    /// When this token expires (None = permanent)
    pub expires_at: Option<DateTime<Utc>>,

    /// BLAKE3 hash of the token secret
    pub token_hash: String,

    /// Whether the token is currently active
    pub active: bool,
}

/// Manages the Proof of Proximity authentication flow.
pub struct ProximityAuth {
    /// All issued tokens
    tokens: Vec<AuthToken>,

    /// Pending authorization requests
    pending: Vec<PendingAuth>,

    /// Node's secret key for signing tokens
    node_secret: [u8; 32],
}

/// A pending authorization request waiting for physical approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAuth {
    pub request_id: Uuid,
    pub device_id: Uuid,
    pub device_name: String,
    pub requested_circle: TrustCircle,
    pub requested_at: DateTime<Utc>,
    /// Expires after 60 seconds if not approved
    pub expires_at: DateTime<Utc>,
}

impl ProximityAuth {
    /// Create a new auth manager.
    pub fn new() -> Self {
        let mut secret = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut secret);

        Self {
            tokens: Vec::new(),
            pending: Vec::new(),
            node_secret: secret,
        }
    }

    /// A device requests authorization. Returns a pending request
    /// that must be approved by physical button press.
    pub fn request_auth(
        &mut self,
        device_id: Uuid,
        device_name: String,
        circle: TrustCircle,
    ) -> PendingAuth {
        let now = Utc::now();
        let pending = PendingAuth {
            request_id: Uuid::new_v4(),
            device_id,
            device_name,
            requested_circle: circle,
            requested_at: now,
            expires_at: now + Duration::seconds(60),
        };

        self.pending.push(pending.clone());
        pending
    }

    /// Physical button was pressed (or NFC tapped).
    /// Approve the oldest pending request and issue a token.
    pub fn approve_pending(&mut self) -> Option<AuthToken> {
        let now = Utc::now();

        // Clean expired pending requests
        self.pending.retain(|p| p.expires_at > now);

        // Approve the first pending request
        if let Some(pending) = self.pending.first().cloned() {
            self.pending.remove(0);
            Some(self.issue_token(
                pending.device_id,
                pending.device_name,
                pending.requested_circle,
            ))
        } else {
            None
        }
    }

    /// Issue a token for a device.
    fn issue_token(
        &mut self,
        device_id: Uuid,
        device_name: String,
        circle: TrustCircle,
    ) -> AuthToken {
        // Generate token secret
        let mut token_bytes = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut token_bytes);

        // Hash with node secret for verification
        let mut hasher = blake3::Hasher::new();
        hasher.update(&token_bytes);
        hasher.update(&self.node_secret);
        let hash = hasher.finalize();

        let expires_at = match circle {
            TrustCircle::Guest => Some(Utc::now() + Duration::hours(24)),
            _ => None, // Permanent for Family and Office
        };

        let token = AuthToken {
            token_id: Uuid::new_v4(),
            device_id,
            device_name,
            circle,
            issued_at: Utc::now(),
            expires_at,
            token_hash: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                hash.as_bytes(),
            ),
            active: true,
        };

        self.tokens.push(token.clone());
        token
    }

    /// Verify a token is valid.
    pub fn verify(&self, token_id: &Uuid) -> Option<&AuthToken> {
        let now = Utc::now();
        self.tokens.iter().find(|t| {
            t.token_id == *token_id
                && t.active
                && t.expires_at.map(|e| e > now).unwrap_or(true)
        })
    }

    /// Revoke a token.
    pub fn revoke(&mut self, token_id: &Uuid) -> bool {
        if let Some(token) = self.tokens.iter_mut().find(|t| t.token_id == *token_id) {
            token.active = false;
            true
        } else {
            false
        }
    }

    /// List all active tokens.
    pub fn list_tokens(&self) -> Vec<&AuthToken> {
        let now = Utc::now();
        self.tokens
            .iter()
            .filter(|t| t.active && t.expires_at.map(|e| e > now).unwrap_or(true))
            .collect()
    }

    /// Get all pending authorization requests.
    pub fn list_pending(&self) -> &[PendingAuth] {
        &self.pending
    }
}

impl Default for ProximityAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_flow() {
        let mut auth = ProximityAuth::new();

        // Device requests access
        let pending = auth.request_auth(
            Uuid::new_v4(),
            "MacBook Pro de Jean".into(),
            TrustCircle::Family,
        );

        assert_eq!(auth.list_pending().len(), 1);

        // Physical button press
        let token = auth.approve_pending().unwrap();

        assert!(token.active);
        assert_eq!(token.circle, TrustCircle::Family);
        assert!(token.expires_at.is_none()); // Family = permanent

        // Verify token
        assert!(auth.verify(&token.token_id).is_some());

        // Revoke
        auth.revoke(&token.token_id);
        assert!(auth.verify(&token.token_id).is_none());
    }

    #[test]
    fn test_guest_token_expires() {
        let mut auth = ProximityAuth::new();
        let _pending = auth.request_auth(
            Uuid::new_v4(),
            "iPhone Visiteur".into(),
            TrustCircle::Guest,
        );
        let token = auth.approve_pending().unwrap();

        assert!(token.expires_at.is_some()); // Guest = 24h expiry
    }
}
