//! Partage du résultat : message prêt à poster et lien « défi » qui rouvre le jeu
//! sur le même scénario (même graine), avec le bilan de l'expéditeur.

use crate::state::GameState;
use serde::{Deserialize, Serialize};

/// À qui l'on partage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Audience {
    /// « À mon DSI » (son ou sa propre DSI, son manager).
    #[serde(rename = "b")]
    Boss,
    /// « À l'équipe de ma DSI ».
    #[serde(rename = "t")]
    Team,
    /// « À un futur DSI ».
    #[serde(rename = "f")]
    Future,
}

impl Audience {
    pub const ALL: [Audience; 3] = [Audience::Boss, Audience::Team, Audience::Future];

    pub fn label(self) -> &'static str {
        match self {
            Audience::Boss => "À mon DSI",
            Audience::Team => "À l'équipe de ma DSI",
            Audience::Future => "À un futur DSI",
        }
    }
}

/// Résultat partagé, transporté dans le lien.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShareCard {
    #[serde(rename = "a")]
    pub audience: Audience,
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "c")]
    pub company: String,
    /// Titre de la fin, ou de la défaite.
    #[serde(rename = "o")]
    pub outcome: String,
    #[serde(rename = "w")]
    pub win: bool,
    #[serde(rename = "s")]
    pub score: u32,
    #[serde(rename = "r")]
    pub rank: String,
    /// Date de jeu (« Décembre — Année 3 »).
    #[serde(rename = "d")]
    pub date: String,
    #[serde(rename = "g")]
    pub seed: u64,
}

/// Construit la carte d'une partie terminée (fin ou défaite).
pub fn card(s: &GameState, audience: Audience) -> ShareCard {
    let (outcome, win, score, rank) = match (&s.ending, s.over) {
        (Some(e), _) => (e.title.clone(), true, e.score, e.rank.clone()),
        (None, Some(o)) => {
            let sc = crate::score::score(s);
            (
                o.title().to_string(),
                false,
                sc,
                crate::score::rank(sc).to_string(),
            )
        }
        (None, None) => {
            let sc = crate::score::score(s);
            (
                "Toujours en poste".into(),
                true,
                sc,
                crate::score::rank(sc).to_string(),
            )
        }
    };
    ShareCard {
        audience,
        name: s.profile.name.clone(),
        company: s.profile.company.clone(),
        outcome,
        win,
        score,
        rank,
        date: s.cal.label(),
        seed: s.seed,
    }
}

/// Message à poster, adapté au destinataire et au résultat.
pub fn message(c: &ShareCard) -> String {
    let (o, s, r, d, co) = (&c.outcome, c.score, &c.rank, &c.date, &c.company);
    match (c.audience, c.win) {
        (Audience::Boss, true) => format!(
            "J'ai survécu comme DSI de {co} jusqu'en {d} : fin « {o} », score {s}/100 (« {r} »). Je dis ça, je dis rien. Même SI hérité, même budget coupé : à toi de jouer."
        ),
        (Audience::Boss, false) => format!(
            "J'ai perdu mon poste de DSI de {co} en {d} : « {o} ». Score {s}/100. Tu ferais mieux ? Même scénario, même batch de 3 h du matin : prouve-le."
        ),
        (Audience::Team, true) => format!(
            "L'équipe, bonne nouvelle : dans DSI — LE JEU, j'ai fini « {o} » avec {s}/100. Personne n'a été remplacé par un agent IA. Enfin, presque. Vous pensez faire mieux que votre DSI ?"
        ),
        (Audience::Team, false) => format!(
            "L'équipe, petite annonce : dans DSI — LE JEU, j'ai fini en « {o} » ({d}). Ce n'est pas votre faute. Enfin, pas que. Revanche sur le même scénario ?"
        ),
        (Audience::Future, true) => format!(
            "Tu vas devenir DSI ? Entraîne-toi d'abord ici. J'ai tenu jusqu'à la fin « {o} », {s}/100 (« {r} »). Le SI date de 1998, le budget vient d'être coupé : bienvenue."
        ),
        (Audience::Future, false) => format!(
            "Futur DSI, apprends de mes erreurs : « {o} » en {d}, score {s}/100. Ça commence toujours par un contrat renouvelé sans relire les conditions. À toi :"
        ),
    }
}

/// Texte d'accueil affiché à la personne qui ouvre le lien.
pub fn invitation(c: &ShareCard) -> (String, String) {
    let who = if c.company.is_empty() {
        c.name.clone()
    } else {
        format!("{}, DSI de {}", c.name, c.company)
    };
    let result = if c.win {
        format!(
            "a terminé la partie sur la fin « {} » ({}), score {}/100 (« {} »)",
            c.outcome, c.date, c.score, c.rank
        )
    } else {
        format!(
            "a perdu son poste en {} : « {} », score {}/100",
            c.date, c.outcome, c.score
        )
    };
    let (title, tail) = match c.audience {
        Audience::Boss => (
            "Un défi de votre collègue",
            "Vous auriez fait mieux ? Même scénario, mêmes aléas.",
        ),
        Audience::Team => (
            "Votre DSI vous défie",
            "Rassurez-vous, ce n'est qu'un jeu. Normalement. Même scénario, mêmes aléas.",
        ),
        Audience::Future => (
            "Entraînement pour futur DSI",
            "Entraînez-vous avant le vrai poste : même scénario, mêmes aléas.",
        ),
    };
    (title.into(), format!("{who} {result}. {tail}"))
}

/// Lien de partage : la page du jeu + `?defi=…`.
pub fn link(base_url: &str, c: &ShareCard) -> String {
    let base = base_url.split(['?', '#']).next().unwrap_or(base_url);
    format!("{base}?defi={}", encode(c))
}

pub fn encode(c: &ShareCard) -> String {
    b64_encode(serde_json::to_string(c).unwrap_or_default().as_bytes())
}

pub fn decode(s: &str) -> Option<ShareCard> {
    let bytes = b64_decode(s.trim())?;
    let c: ShareCard = serde_json::from_slice(&bytes).ok()?;
    // Bornes de sécurité : le contenu vient d'un lien, on ne lui fait pas confiance.
    let ok = c.name.chars().count() <= 60
        && c.company.chars().count() <= 80
        && c.outcome.chars().count() <= 80
        && c.rank.chars().count() <= 60
        && c.date.chars().count() <= 40
        && c.score <= 100;
    ok.then_some(c)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Base64 « URL safe », sans remplissage.
fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        let chars = chunk.len() + 1;
        for i in 0..chars {
            out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| B64.iter().position(|&b| b == c).map(|p| p as u32);
    let bytes: Vec<u32> = s
        .bytes()
        .filter(|b| *b != b'=')
        .map(val)
        .collect::<Option<_>>()?;
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.len() == 1 {
            return None;
        }
        let mut n = 0u32;
        for (i, v) in chunk.iter().enumerate() {
            n |= v << (18 - 6 * i);
        }
        for i in 0..chunk.len() - 1 {
            out.push((n >> (16 - 8 * i) & 255) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_aller_retour() {
        for s in ["", "a", "ab", "abc", "abcd", "Déjà vu — SI hérité ✔"] {
            assert_eq!(b64_decode(&b64_encode(s.as_bytes())).unwrap(), s.as_bytes());
        }
        assert!(b64_decode("a").is_none());
        assert!(b64_decode("@@@@").is_none());
    }
}
