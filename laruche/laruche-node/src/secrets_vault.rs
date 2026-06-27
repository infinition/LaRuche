//! Coffre à secrets **chiffré au repos** (côté node).
//!
//! Stocke `NOM → valeur` chiffrés dans `secrets.enc`. La clé maîtresse (32 octets aléatoires)
//! vit dans `secret.key` (générée au 1er lancement). Chiffrement : keystream dérivé de
//! **blake3 en mode keyed** (PRF) XORé au clair, avec un nonce aléatoire par secret — pas
//! d'AES (aucune dépendance ajoutée), mais un vrai chiffrement par flux au repos.
//!
//! Les valeurs déchiffrées ne vivent qu'en mémoire (poussées dans `laruche_essaim::secrets`)
//! et ne sont JAMAIS renvoyées par les endpoints (seuls les NOMS sortent).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

const FICHIER: &str = "secrets.enc";
const FICHIER_CLE: &str = "secret.key";

fn charger_cle() -> [u8; 32] {
    let p = Path::new(FICHIER_CLE);
    if let Ok(bytes) = std::fs::read(p) {
        if bytes.len() == 32 {
            let mut k = [0u8; 32];
            k.copy_from_slice(&bytes);
            return k;
        }
    }
    // Génère et persiste une nouvelle clé maîtresse.
    let k: [u8; 32] = rand::random();
    let _ = std::fs::write(p, k);
    k
}

/// Keystream blake3-keyed(clé, nonce) de la longueur voulue, XORé au clair → chiffré.
fn xor_flux(cle: &[u8; 32], nonce: &[u8], donnee: &[u8]) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new_keyed(cle);
    hasher.update(nonce);
    let mut xof = hasher.finalize_xof();
    let mut flux = vec![0u8; donnee.len()];
    xof.fill(&mut flux);
    donnee.iter().zip(flux.iter()).map(|(a, b)| a ^ b).collect()
}

fn chiffrer(cle: &[u8; 32], clair: &str) -> String {
    let nonce: [u8; 16] = rand::random();
    let chiffre = xor_flux(cle, &nonce, clair.as_bytes());
    // format: base64(nonce) ":" base64(chiffré)
    format!("{}:{}", b64(&nonce), b64(&chiffre))
}

fn dechiffrer(cle: &[u8; 32], blob: &str) -> Option<String> {
    let (n, c) = blob.split_once(':')?;
    let nonce = unb64(n)?;
    let chiffre = unb64(c)?;
    let clair = xor_flux(cle, &nonce, &chiffre);
    String::from_utf8(clair).ok()
}

// Base64 minimal (sans dépendance) — alphabet standard.
fn b64(data: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(A[((n >> 18) & 63) as usize] as char);
        out.push(A[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { A[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { A[(n & 63) as usize] as char } else { '=' });
    }
    out
}

fn unb64(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let s: Vec<u8> = s.bytes().filter(|&c| c != b'=' && !c.is_ascii_whitespace()).collect();
    let mut out = Vec::new();
    for chunk in s.chunks(4) {
        let mut n = 0u32;
        let mut bits = 0;
        for &c in chunk {
            n = (n << 6) | val(c)?;
            bits += 6;
        }
        n <<= 24 - bits;
        let nbytes = bits / 8;
        for i in 0..nbytes {
            out.push((n >> (16 - i * 8)) as u8);
        }
    }
    Some(out)
}

fn chemin() -> PathBuf {
    PathBuf::from(FICHIER)
}

/// Charge et déchiffre tous les secrets depuis le disque. Map `NOM → valeur claire`.
pub fn charger() -> HashMap<String, String> {
    let cle = charger_cle();
    let mut out = HashMap::new();
    if let Ok(raw) = std::fs::read_to_string(chemin()) {
        if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&raw) {
            for (nom, blob) in map {
                if let Some(val) = dechiffrer(&cle, &blob) {
                    out.insert(nom, val);
                }
            }
        }
    }
    out
}

/// Persiste la map claire `NOM → valeur` en la chiffrant. Best-effort.
pub fn sauver(map: &HashMap<String, String>) {
    let cle = charger_cle();
    let chiffre: HashMap<String, String> =
        map.iter().map(|(n, v)| (n.clone(), chiffrer(&cle, v))).collect();
    if let Ok(json) = serde_json::to_string_pretty(&chiffre) {
        let _ = std::fs::write(chemin(), json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b64_roundtrip() {
        for s in ["", "a", "ab", "abc", "abcd", "hello world!", "tok_9aZ+/x"] {
            let e = b64(s.as_bytes());
            assert_eq!(unb64(&e).unwrap(), s.as_bytes(), "roundtrip {s}");
        }
    }

    #[test]
    fn chiffre_dechiffre_roundtrip() {
        let cle = [7u8; 32];
        let blob = chiffrer(&cle, "secret_token_123");
        assert!(!blob.contains("secret_token_123"), "le clair ne doit pas apparaitre");
        assert_eq!(dechiffrer(&cle, &blob).unwrap(), "secret_token_123");
    }
}
