//! # platform
//!
//! Fines couches d'adaptation autour des services de la plateforme, pour que le jeu
//! ne touche jamais directement une API du navigateur ou du système :
//!
//! * [`storage`] — persistance clé/valeur (localStorage sur le web, fichiers sur desktop).
//!   Un futur backend n'aurait qu'à fournir une autre implémentation de `Storage`.
//! * [`clock`] — horloge murale pour tirer la graine des nouvelles parties.
//! * [`params`] — paramètres d'URL (`?seed=1234`, `?debug=true`) ou variables d'environnement.

pub mod storage;

#[cfg(target_arch = "wasm32")]
pub mod web;

pub mod clock {
    /// Millisecondes depuis l'époque Unix.
    pub fn now_millis() -> u64 {
        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Date::now() as u64
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        }
    }
}

pub mod params {
    /// Paramètres de lancement.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct LaunchParams {
        pub seed: Option<u64>,
        pub debug: bool,
        /// Lien de défi reçu (`?defi=…`), encodé.
        pub defi: Option<String>,
    }

    /// Analyse une query string du type `?seed=1234&debug=true`.
    pub fn parse(query: &str) -> LaunchParams {
        let mut p = LaunchParams::default();
        for pair in query.trim_start_matches('?').split('&') {
            let mut it = pair.splitn(2, '=');
            let k = it.next().unwrap_or("");
            let v = it.next().unwrap_or("");
            match k {
                "seed" => p.seed = v.parse().ok(),
                "debug" => p.debug = matches!(v, "true" | "1" | "yes" | ""),
                "defi" if !v.is_empty() => p.defi = Some(v.to_string()),
                _ => {}
            }
        }
        p
    }

    pub fn current() -> LaunchParams {
        #[cfg(target_arch = "wasm32")]
        {
            parse(&crate::web::query_string())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut q = String::new();
            if let Ok(s) = std::env::var("DSI_SEED") {
                q.push_str(&format!("seed={s}&"));
            }
            if std::env::var("DSI_DEBUG").is_ok() {
                q.push_str("debug=true");
            }
            parse(&q)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn parse_query() {
            assert_eq!(
                parse("?seed=1234&debug=true"),
                LaunchParams {
                    seed: Some(1234),
                    debug: true,
                    defi: None
                }
            );
            assert_eq!(
                parse("?debug=false"),
                LaunchParams {
                    seed: None,
                    debug: false,
                    defi: None
                }
            );
            assert_eq!(parse("?defi=eyJh").defi.as_deref(), Some("eyJh"));
            assert_eq!(parse(""), LaunchParams::default());
        }
    }
}

pub use storage::{MemoryStorage, Storage, StorageError, default_storage};
