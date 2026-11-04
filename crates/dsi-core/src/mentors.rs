//! Les mentors d'A la French : Mehdi, Steeve et Jean-Baptiste.
//!
//! * À la création de la partie, le joueur choisit un **mentor principal** : son bonus
//!   permanent s'applique une fois, et il répond deux fois plus souvent.
//! * Pendant la partie, chaque mentor peut être **consulté** (commande
//!   `ConsultMentor`, gratuite en agenda) : il donne le premier conseil de sa liste dont
//!   les conditions sont remplies, avec un petit bonus. Puis il faut attendre son délai.

use crate::conditions::Conditions;
use crate::content::Content;
use crate::effects::{Effects, Outcome};
use crate::state::{GameState, Resolution, Tone};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AdviceDef {
    #[serde(default)]
    pub conditions: Conditions,
    pub text: String,
    #[serde(flatten)]
    pub outcome: Outcome,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MentorDef {
    pub id: String,
    pub name: String,
    pub title: String,
    pub bio: String,
    #[serde(default)]
    pub skills: Vec<String>,
    /// Bonus permanent appliqué une fois quand il devient mentor principal.
    #[serde(default)]
    pub passive: Effects,
    pub passive_label: String,
    /// Délai entre deux consultations (mois) : principal / autres.
    pub cooldown_main: u32,
    pub cooldown: u32,
    /// Couleur d'accent (hex).
    #[serde(default)]
    pub color: String,
    /// Phrase d'accueil du tutoriel.
    #[serde(default)]
    pub hello: String,
    pub advice: Vec<AdviceDef>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MentorState {
    /// Mois absolu de la dernière consultation de chaque mentor.
    #[serde(default)]
    pub last_consult: BTreeMap<String, u32>,
    /// Dernier conseil reçu de chaque mentor.
    #[serde(default)]
    pub last_advice: BTreeMap<String, String>,
    #[serde(default)]
    pub consults: u32,
}

pub fn cooldown(s: &GameState, def: &MentorDef) -> u32 {
    if s.profile.mentor == def.id { def.cooldown_main } else { def.cooldown }
}

/// Mois restants avant de pouvoir consulter ce mentor (0 = disponible).
pub fn wait(s: &GameState, def: &MentorDef) -> u32 {
    match s.mentors.last_consult.get(&def.id) {
        Some(&t) => (t + cooldown(s, def)).saturating_sub(s.turn()),
        None => 0,
    }
}

/// Le conseil que ce mentor donnerait maintenant.
pub fn advice<'a>(s: &GameState, def: &'a MentorDef) -> Option<&'a AdviceDef> {
    def.advice.iter().find(|a| a.conditions.met(s))
}

/// Applique le bonus permanent du mentor principal (nouvelle partie).
pub fn apply_passive(content: &Content, s: &mut GameState) {
    if let Some(def) = content.mentor(&s.profile.mentor).cloned() {
        crate::effects::apply(s, &def.passive);
        s.flags.insert(format!("mentor_{}", def.id));
        s.push_log(format!("{} devient votre mentor : {}.", def.name, def.passive_label), Tone::Good);
    }
}

pub fn consult(content: &Content, s: &mut GameState, id: &str) -> Result<Resolution, String> {
    let def = content.mentor(id).cloned().ok_or_else(|| format!("mentor inconnu : {id}"))?;
    let w = wait(s, &def);
    if w > 0 {
        return Err(format!("{} est en tournage du podcast. Disponible dans {w} mois.", def.name));
    }
    let a = advice(s, &def).cloned().ok_or_else(|| format!("{} n'a rien à ajouter pour l'instant.", def.name))?;
    let now = s.turn();
    s.mentors.last_consult.insert(def.id.clone(), now);
    s.mentors.last_advice.insert(def.id.clone(), a.text.clone());
    s.mentors.consults += 1;
    let (deltas, notes, _, _) = crate::game::apply_outcome(content, s, &a.outcome, &format!("mentor:{}", def.id));
    s.push_log(format!("{} : « {} »", def.name, a.text), Tone::Story);
    s.decisions.push(format!("mentor:{}", def.id));
    let mut notes = notes;
    if a.outcome.set_flag.iter().any(|f| f == "mentor_leverage") {
        notes.push("Bonus : +10 de rapport de force à votre prochaine négociation.".into());
    }
    notes.push(format!("Prochain conseil de {} dans {} mois.", def.name, cooldown(s, &def)));
    Ok(Resolution { title: format!("Conseil de {}", def.name), text: a.text, deltas, notes, delayed_hint: false, speaker: format!("mentor:{}", def.id) })
}
