//! Partage vers les réseaux sociaux : liens d'intention, presse-papiers et partage natif.

/// Adresse publique du jeu (repli hors navigateur).
pub const PUBLIC_URL: &str = "https://a-la-french.github.io/dsi-le-jeu/";

#[cfg(target_arch = "wasm32")]
mod js {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(inline_js = "
export function dsi_page_url() { return window.location.origin + window.location.pathname; }
export function dsi_open(url) { window.open(url, '_blank', 'noopener'); }
export function dsi_copy(text) {
  try {
    if (navigator.clipboard && window.isSecureContext) { navigator.clipboard.writeText(text); return true; }
  } catch (e) {}
  try {
    const t = document.createElement('textarea');
    t.value = text; t.style.position = 'fixed'; t.style.opacity = '0';
    document.body.appendChild(t); t.select();
    const ok = document.execCommand('copy');
    document.body.removeChild(t);
    return ok;
  } catch (e) { return false; }
}
export function dsi_can_share() { return !!navigator.share; }
export function dsi_share(title, text, url) {
  if (!navigator.share) return false;
  navigator.share({ title, text, url }).catch(() => {});
  return true;
}
")]
    extern "C" {
        pub fn dsi_page_url() -> String;
        pub fn dsi_open(url: &str);
        pub fn dsi_copy(text: &str) -> bool;
        pub fn dsi_can_share() -> bool;
        pub fn dsi_share(title: &str, text: &str, url: &str) -> bool;
    }
}

/// Adresse de la page du jeu (sans paramètres).
pub fn page_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let u = js::dsi_page_url();
        if u.starts_with("http") {
            return u;
        }
    }
    PUBLIC_URL.to_string()
}

pub fn open(url: &str) {
    #[cfg(target_arch = "wasm32")]
    js::dsi_open(url);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = url;
}

pub fn copy(text: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        js::dsi_copy(text)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = text;
        false
    }
}

pub fn can_native_share() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        js::dsi_can_share()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

pub fn native_share(title: &str, text: &str, url: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        js::dsi_share(title, text, url)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (title, text, url);
        false
    }
}

/// Encodage pour paramètre d'URL (RFC 3986, caractères non réservés conservés).
pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Réseaux proposés : (nom, adresse de partage).
pub fn networks(text: &str, url: &str) -> Vec<(&'static str, String)> {
    let full = format!("{text} {url}");
    vec![
        (
            "LinkedIn",
            format!(
                "https://www.linkedin.com/sharing/share-offsite/?url={}",
                url_encode(url)
            ),
        ),
        (
            "X",
            format!(
                "https://twitter.com/intent/tweet?text={}&url={}",
                url_encode(text),
                url_encode(url)
            ),
        ),
        (
            "Bluesky",
            format!("https://bsky.app/intent/compose?text={}", url_encode(&full)),
        ),
        (
            "WhatsApp",
            format!("https://wa.me/?text={}", url_encode(&full)),
        ),
        (
            "E-mail",
            format!(
                "mailto:?subject={}&body={}",
                url_encode("DSI — LE JEU : mon bilan"),
                url_encode(&full)
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodage_url() {
        assert_eq!(url_encode("a b&é"), "a%20b%26%C3%A9");
        let n = networks("Fin « Software Company »", "https://x.fr/?defi=ab");
        assert_eq!(n.len(), 5);
        assert!(n.iter().all(|(_, u)| !u.contains(' ')));
    }
}
