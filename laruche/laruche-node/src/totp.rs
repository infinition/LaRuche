//! Minimal TOTP (RFC 6238 / RFC 4226) for optional 2FA on top of the password login.
//! Self-contained: base32 + HMAC-SHA1 (via the cached sha1_smol crate), no extra crypto deps.

use sha1_smol::Sha1;

const B32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Generate a random base32 secret (160 bits = 32 chars), suitable for an authenticator app.
pub fn generate_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 20];
    rand::thread_rng().fill_bytes(&mut bytes);
    base32_encode(&bytes)
}

pub fn base32_encode(data: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for &b in data {
        buffer = (buffer << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[((buffer >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(B32[((buffer << (5 - bits)) & 0x1f) as usize] as char);
    }
    out
}

pub fn base32_decode(s: &str) -> Option<Vec<u8>> {
    let mut buffer = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::new();
    for c in s.chars().filter(|c| !c.is_whitespace() && *c != '=') {
        let c = c.to_ascii_uppercase();
        let val = B32.iter().position(|&b| b as char == c)? as u32;
        buffer = (buffer << 5) | val;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h = Sha1::new();
    h.update(data);
    h.digest().bytes()
}

fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..20].copy_from_slice(&sha1(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let mut inner = Sha1::new();
    inner.update(&ipad);
    inner.update(msg);
    let inner_d = inner.digest().bytes();
    let mut outer = Sha1::new();
    outer.update(&opad);
    outer.update(&inner_d);
    outer.digest().bytes()
}

/// HOTP code for a counter (6 digits, RFC 4226 dynamic truncation).
fn hotp(secret: &[u8], counter: u64) -> u32 {
    let hash = hmac_sha1(secret, &counter.to_be_bytes());
    let offset = (hash[19] & 0x0f) as usize;
    let bin = ((hash[offset] as u32 & 0x7f) << 24)
        | ((hash[offset + 1] as u32) << 16)
        | ((hash[offset + 2] as u32) << 8)
        | (hash[offset + 3] as u32);
    bin % 1_000_000
}

/// Verify a 6-digit TOTP code against a base32 secret, allowing +/- one 30s step for clock drift.
pub fn verify(secret_b32: &str, code: &str, unix_time: u64) -> bool {
    let code: u32 = match code.trim().parse() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let secret = match base32_decode(secret_b32) {
        Some(s) => s,
        None => return false,
    };
    let counter = (unix_time / 30) as i64;
    for delta in [-1i64, 0, 1] {
        if hotp(&secret, (counter + delta) as u64) == code {
            return true;
        }
    }
    false
}

/// otpauth:// URL for the authenticator-app QR code.
pub fn otpauth_url(secret_b32: &str, account: &str, issuer: &str) -> String {
    let acct = urlencode(account);
    let iss = urlencode(issuer);
    format!(
        "otpauth://totp/{iss}:{acct}?secret={secret_b32}&issuer={iss}&algorithm=SHA1&digits=6&period=30"
    )
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotp_rfc4226_vectors() {
        // RFC 4226 Appendix D: secret = ASCII "12345678901234567890".
        let secret = b"12345678901234567890";
        assert_eq!(hotp(secret, 0), 755224);
        assert_eq!(hotp(secret, 1), 287082);
        assert_eq!(hotp(secret, 2), 359152);
        assert_eq!(hotp(secret, 9), 520489);
    }

    #[test]
    fn base32_roundtrip() {
        for data in [&b"Hello!"[..], &b"12345678901234567890"[..], &[0u8, 255, 1, 2]] {
            let enc = base32_encode(data);
            assert_eq!(base32_decode(&enc).unwrap(), data);
        }
    }

    #[test]
    fn verify_accepts_current_window() {
        let secret = base32_encode(b"12345678901234567890");
        // T=59 -> counter 1 -> 6-digit code 287082.
        assert!(verify(&secret, "287082", 59));
        assert!(!verify(&secret, "000000", 59));
    }
}
