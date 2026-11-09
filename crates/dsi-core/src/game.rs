//! L'API du jeu : nouvelle partie, commandes du joueur, passage du mois.
//! Le client n'écrit jamais l'état directement : il envoie des `Command`.

use crate::content::Content;
use crate::contracts::ContractState;
use crate::effects::{self, ChoiceDef, Effects, Minigame, Outcome};
use crate::events;
use crate::state::{
    Active, Calendar, GameState, Inbox, Metric, Profile, Resolution, Room, Scheduled, Tone,
};
use crate::{ai, cloud, economy, negotiation, people, projects, score, security, technology};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Command {
    /// Ouvre l'élément n° i de la boîte de réception.
    Open(usize),
    /// Choisit l'option i de la scène active.
    Choose(usize),
    /// Ferme un résultat / un chapitre / une revue.
    Continue,
    AdvanceMonth,
    StartProject(String),
    EsnBoost(String),
    CancelProject(String),
    DoAction(String),
    Negotiate(String),
    Recruit(String),
    /// Licencie la recrue d'indice donné dans `team` (1 créneau d'agenda).
    Dismiss(usize),
    PlayCard(String),
    Conclude,
    AcceptOffer,
    Answer(usize),
    MakeOffer(Option<usize>),
    War(security::WarAction),
    WarClose,
    // Mode debug.
    DebugSet(Metric, f32),
    DebugEvent(String),
    DebugTech(String),
    DebugAgenda,
    DebugSkip,
    DebugHire(String),
    /// Demande un conseil à un mentor (gratuit en agenda, délai entre deux conseils).
    ConsultMentor(String),
    /// Après une fin de partie : continuer 3 années de plus.
    ExtendCampaign,
    /// Saison 2 : choisit la fin visée (le « cap »), ou aucune.
    /// Le premier choix est libre ; en changer ensuite coûte un créneau et un peu de COMEX.
    SetAmbition(Option<String>),
    /// Règle l'étape du tutoriel (voir `state::TUTORIAL_*`).
    SetTutorial(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum GameError {
    Busy,
    NotNow(String),
    NoAgenda,
    Unknown(String),
    Finished,
}

impl GameError {
    pub fn label(&self) -> String {
        match self {
            GameError::Busy => "Une décision est en attente.".into(),
            GameError::NotNow(s) => s.clone(),
            GameError::NoAgenda => {
                "Plus de créneau d'agenda ce mois-ci. Votre assistante a bloqué vos déjeuners."
                    .into()
            }
            GameError::Unknown(s) => format!("Inconnu : {s}"),
            GameError::Finished => "La partie est terminée.".into(),
        }
    }
}

pub type GameResult = Result<(), GameError>;

/// Crée une nouvelle partie.
pub fn new_game(content: &Content, seed: u64, profile: Profile) -> GameState {
    let mut s = GameState::new(seed, profile);
    if let Some(bg) = content
        .dialogues
        .backgrounds
        .iter()
        .find(|b| b.id == s.profile.background)
    {
        effects::apply(&mut s, &bg.effects);
    }
    for t in content.techs.iter().filter(|t| t.initial) {
        s.techs.insert(t.id.clone());
    }
    for c in content.contracts.iter().filter(|c| c.initial) {
        s.contracts.push(ContractState::from_def(c, 0));
    }
    s.baseline_run = score::run_cost(&s);
    crate::mentors::apply_passive(content, &mut s);
    s.push_log(s.tx("{dsi} prend ses fonctions de DSI chez {company}. Badge visiteur fourni « en attendant »."), Tone::Story);
    begin_month(content, &mut s);
    s
}

pub fn add_contract(content: &Content, s: &mut GameState, id: &str) {
    let Some(d) = content.contract(id) else {
        return;
    };
    let now = s.turn();
    if let Some(c) = s.contracts.iter_mut().find(|c| c.id == id) {
        if !c.active {
            *c = ContractState::from_def(d, now);
            if d.first_renewal.is_some_and(|a| {
                Calendar {
                    year: a.year,
                    month: a.month,
                }
                .abs()
                    <= now
            }) {
                c.next_renewal = now + d.term;
            }
        }
        return;
    }
    let mut c = ContractState::from_def(d, now);
    if c.next_renewal <= now {
        c.next_renewal = now + d.term;
    }
    s.contracts.push(c);
}

/// Salle où se déroule un élément de la boîte de réception.
pub fn inbox_room(content: &Content, item: &Inbox) -> Room {
    match item {
        Inbox::Event { id } => content.event(id).map(|e| e.room).unwrap_or_default(),
        Inbox::Consequence { room, .. } => *room,
        Inbox::Renewal { .. } => Room::Vendor,
        Inbox::YearReview => Room::Comex,
        Inbox::Chapter { .. } => Room::Dsi,
    }
}

pub fn inbox_title(content: &Content, s: &GameState, item: &Inbox) -> String {
    match item {
        Inbox::Event { id } => content
            .event(id)
            .map(|e| s.tx(&e.title))
            .unwrap_or_else(|| id.clone()),
        Inbox::Consequence { title, .. } => title.clone(),
        Inbox::Renewal { contract } => format!(
            "Renouvellement : {}",
            content
                .contract(contract)
                .map(|c| c.label.clone())
                .unwrap_or_default()
        ),
        Inbox::YearReview => "COMEX annuel".into(),
        Inbox::Chapter { index } => content
            .dialogues
            .chapters
            .get(*index)
            .map(|c| c.title.clone())
            .unwrap_or_default(),
    }
}

/// Début d'un mois : chapitre, renouvellements, conséquences, événements scriptés et aléatoires.
fn begin_month(content: &Content, s: &mut GameState) {
    let now = s.turn();
    s.agenda = s.agenda_max;
    // Chapitre.
    if let Some((i, _)) = content
        .dialogues
        .chapters
        .iter()
        .enumerate()
        .find(|(_, c)| c.at.year == s.cal.year && c.at.month == s.cal.month)
    {
        s.inbox.push(Inbox::Chapter { index: i });
    }
    // Renouvellements.
    let due: Vec<(String, Option<f32>)> = s
        .contracts
        .iter()
        .filter(|c| c.active && c.next_renewal <= now)
        .map(|c| (c.id.clone(), c.pre_deal))
        .collect();
    for (id, pre) in due {
        let def = content.contract(&id).cloned();
        if let (Some(inc), Some(def)) = (pre, def) {
            if let Some(c) = s.contracts.iter_mut().find(|c| c.id == id) {
                c.monthly_cost *= 1.0 + inc;
                c.next_renewal = now + def.term;
                c.pre_deal = None;
                c.renewals += 1;
            }
            s.push_log(
                format!(
                    "{} renouvelé selon le pré-accord ({:+.0} %).",
                    def.label,
                    inc * 100.0
                ),
                Tone::Money,
            );
        } else {
            s.inbox.push(Inbox::Renewal { contract: id });
        }
    }
    // Conséquences programmées.
    let (due, rest): (Vec<Scheduled>, Vec<Scheduled>) = std::mem::take(&mut s.scheduled)
        .into_iter()
        .partition(|x| x.due <= now);
    s.scheduled = rest;
    for d in due {
        if let Some(e) = d.event {
            if content.event(&e).is_some()
                && !s
                    .inbox
                    .iter()
                    .any(|i| matches!(i, Inbox::Event { id } if *id == e))
            {
                s.inbox.push(Inbox::Event { id: e });
            }
        } else {
            let room = if d.effects.contains_key(&Metric::Security) {
                Room::Soc
            } else {
                Room::Dsi
            };
            s.inbox.push(Inbox::Consequence {
                title: "Conséquence".into(),
                text: d.text,
                effects: d.effects,
                room,
            });
        }
    }
    // Événements scriptés puis aléatoires.
    let scripted = events::scripted_now(content, s);
    let n_scripted = scripted.len();
    for id in scripted {
        s.inbox.push(Inbox::Event { id });
    }
    if now > 0 {
        let chaos = (s.m(Metric::Debt) + s.m(Metric::Complexity)) / 400.0;
        let mut n = if n_scripted > 0 {
            usize::from(s.rng.random::<f32>() < 0.55 + chaos * 0.3)
        } else {
            1 + usize::from(s.rng.random::<f32>() < 0.3 + chaos)
        };
        if s.inbox.len() >= 4 {
            n = n.min(1);
        }
        for id in events::draw_random(content, s, n) {
            s.inbox.push(Inbox::Event { id });
        }
    }
    // Ambiance.
    if !content.dialogues.ambient.is_empty() && s.rng.random::<f32>() < 0.6 {
        let l = content.dialogues.ambient[s.rng.random_range(0..content.dialogues.ambient.len())]
            .clone();
        s.push_log(l, Tone::Info);
    }
}

/// Le mois qui se termine : économie, technos, équipe, cloud, IA, projets.
fn end_month(content: &Content, s: &mut GameState) -> Vec<String> {
    economy::monthly(content, s);
    technology::monthly(content, s);
    people::monthly(content, s);
    cloud::monthly(content, s);
    ai::monthly(s);
    let evs = projects::monthly(content, s);
    if !content.dialogues.daf_lines.is_empty() && s.rng.random::<f32>() < 0.35 {
        let l = content.dialogues.daf_lines
            [s.rng.random_range(0..content.dialogues.daf_lines.len())]
        .clone();
        s.push_log(format!("DAF : « {l} »"), Tone::Money);
    }
    s.check_game_over();
    evs
}

fn ensure_free(s: &GameState) -> GameResult {
    if s.finished() {
        return Err(GameError::Finished);
    }
    if s.active.is_some() {
        return Err(GameError::Busy);
    }
    Ok(())
}

fn spend_agenda(s: &mut GameState, n: u32) -> GameResult {
    if s.agenda < n {
        return Err(GameError::NoAgenda);
    }
    s.agenda -= n;
    Ok(())
}

/// Applique le résultat d'un choix. Renvoie (variations, notes, mini-jeu éventuel).
pub fn apply_outcome(
    content: &Content,
    s: &mut GameState,
    o: &Outcome,
    origin: &str,
) -> (Vec<(Metric, f32)>, Vec<String>, Option<Minigame>, bool) {
    let mut deltas = effects::apply(s, &o.effects);
    let mut notes = Vec::new();
    let mut hidden = false;
    if o.allocation != 0.0 {
        s.allocation = (s.allocation + o.allocation).max(0.5);
        notes.push(format!(
            "Allocation mensuelle : {:+.1} pt/mois (désormais {:.1}).",
            o.allocation, s.allocation
        ));
    }
    if o.agenda != 0 {
        s.agenda = (s.agenda as i32 + o.agenda).clamp(0, 6) as u32;
        notes.push(if o.agenda < 0 {
            format!("{} créneau(x) d'agenda consommé(s).", -o.agenda)
        } else {
            format!("+{} créneau(x) d'agenda.", o.agenda)
        });
    }
    if let Some(cc) = &o.contract_cost
        && let Some(c) = s
            .contracts
            .iter_mut()
            .find(|c| c.id == cc.contract && c.active)
    {
        c.monthly_cost = (c.monthly_cost + cc.cost).max(0.0);
        notes.push(format!(
            "Contrat {} : {:+.2} pt/mois.",
            content
                .contract(&cc.contract)
                .map(|d| d.label.as_str())
                .unwrap_or(""),
            cc.cost
        ));
    }
    for c in &o.end_contract {
        if let Some(x) = s.contracts.iter_mut().find(|x| &x.id == c && x.active) {
            x.active = false;
            notes.push(format!(
                "Contrat résilié : {}.",
                content.contract(c).map(|d| d.label.as_str()).unwrap_or(c)
            ));
        }
    }
    for c in &o.add_contract {
        add_contract(content, s, c);
        notes.push(format!(
            "Nouveau contrat : {}.",
            content.contract(c).map(|d| d.label.as_str()).unwrap_or(c)
        ));
    }
    for t in &o.add_tech {
        if s.techs.insert(t.clone()) {
            notes.push(format!("Technologie : + {}", content.tech_name(t)));
        }
    }
    for t in &o.remove_tech {
        if s.techs.remove(t) {
            notes.push(format!("Technologie : − {}", content.tech_name(t)));
        }
    }
    for f in &o.set_flag {
        s.flags.insert(f.clone());
    }
    for f in &o.clear_flag {
        s.flags.remove(f);
    }
    if let Some(p) = &o.start_project {
        match projects::can_start(content, s, p) {
            Ok(()) => {
                effects::merge(&mut deltas, projects::start(content, s, p));
                notes.push(format!(
                    "Projet lancé : {}.",
                    content.project(p).map(|d| d.name.as_str()).unwrap_or(p)
                ));
            }
            Err(e) => notes.push(format!("Projet non lancé ({e}).")),
        }
    }
    if o.lose_hire && !s.team.is_empty() {
        let i = s.rng.random_range(0..s.team.len());
        let h = s.team.remove(i);
        notes.push(format!("{} quitte l'équipe.", h.name));
    }
    if let Some(p) = &o.cloud_provider {
        s.cloud.provider = Some(p.clone());
    }
    if o.cloud_bill != 0.0 {
        s.cloud.bill = (s.cloud.bill + o.cloud_bill).max(0.0);
        notes.push(format!("Facture cloud : {:+.2} pt/mois.", o.cloud_bill));
    }
    if o.finops != 0.0 {
        s.cloud.finops = (s.cloud.finops + o.finops).clamp(0.0, 100.0);
    }
    s.ai.code_generated += o.code_generated;
    s.ai.code_understood += o.code_understood;
    let now = s.turn();
    for d in &o.delayed {
        let mut p = d.chance;
        for (m, k) in &d.chance_by {
            p += k * s.m(*m) / 100.0;
        }
        hidden = true;
        if s.rng.random::<f32>() < p.clamp(0.0, 1.0) {
            let jitter = if d.jitter > 0 {
                s.rng.random_range(0..=d.jitter)
            } else {
                0
            };
            s.scheduled.push(Scheduled {
                due: now + d.months.max(1) + jitter,
                event: d.event.clone(),
                effects: d.effects.clone(),
                text: s.tx(&d.text),
                origin: origin.into(),
            });
        }
    }
    s.check_game_over();
    (deltas, notes, o.minigame.clone(), hidden)
}

/// Résout un choix (événement ou action) et renvoie la scène suivante.
fn resolve_choice(
    content: &Content,
    s: &mut GameState,
    title: &str,
    ch: &ChoiceDef,
    origin: &str,
) -> Option<Active> {
    let mut outcome = ch.outcome.clone();
    let mut extra = String::new();
    if let Some(g) = &ch.gamble {
        let p = g.probability(s);
        let win = s.rng.random::<f32>() < p;
        let o = if win { &g.win } else { &g.lose };
        outcome.absorb(o);
        extra = o.result.clone();
    }
    let (deltas, notes, mg, hidden) = apply_outcome(content, s, &outcome, origin);
    if let Some(c) = &outcome.chain
        && content.event(c).is_some()
    {
        s.inbox.insert(0, Inbox::Event { id: c.clone() });
    }
    s.decisions.push(format!("{origin}:{}", ch.label));
    s.push_log(
        format!("{title} → {}", s.tx(&ch.label)),
        crate::state::Tone::Story,
    );
    if let Some(m) = mg {
        return Some(match m {
            Minigame::Negotiation { contract } => {
                match negotiation::start(content, s, &contract, true) {
                    Some(n) => Active::Negotiation(Box::new(n)),
                    None => result(
                        title,
                        "Le contrat n'est plus actif : rien à négocier.",
                        deltas,
                        notes,
                        hidden,
                    ),
                }
            }
            Minigame::Recruit { role } => match people::start(content, s, &role) {
                Some(iv) => Active::Interview(Box::new(iv)),
                None => result(
                    title,
                    "Aucun candidat. La DRH propose de republier l'annonce sur Monster.",
                    deltas,
                    notes,
                    hidden,
                ),
            },
            Minigame::WarRoom { scenario } => match security::start(content, s, &scenario) {
                Some(w) => Active::WarRoom(Box::new(w)),
                None => result(title, "", deltas, notes, hidden),
            },
        });
    }
    let mut text = s.tx(&outcome.result);
    if !extra.is_empty() {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(&s.tx(&extra));
    }
    Some(result(title, &text, deltas, notes, hidden))
}

fn result(
    title: &str,
    text: &str,
    deltas: Vec<(Metric, f32)>,
    notes: Vec<String>,
    hidden: bool,
) -> Active {
    Active::Result(Box::new(Resolution {
        title: title.into(),
        text: text.into(),
        deltas,
        notes,
        speaker: String::new(),
        delayed_hint: hidden,
    }))
}

/// Options d'un renouvellement de contrat (construites à partir des données).
pub fn renewal_choices(
    content: &Content,
    s: &GameState,
    contract: &str,
) -> Vec<(String, String, bool)> {
    let Some(def) = content.contract(contract) else {
        return vec![];
    };
    let mut v = vec![
        (
            "Négocier".to_string(),
            "Mini-jeu de négociation. Vous êtes à J-2 : le rapport de force part bas.".to_string(),
            true,
        ),
        (
            format!("Accepter la hausse (+{:.0} %)", def.increase * 100.0),
            "Rapide. Le DAF va adorer. Non.".to_string(),
            true,
        ),
    ];
    for p in &def.migration {
        if let Some(pd) = content.project(p) {
            let ok = projects::can_start(content, s, p).is_ok();
            v.push((
                format!("Prolonger 12 mois et lancer : {}", pd.name),
                format!("Hausse réduite de moitié le temps de migrer. {}", pd.desc),
                ok,
            ));
        }
    }
    v
}

pub fn apply(content: &Content, s: &mut GameState, cmd: Command) -> GameResult {
    if s.over.is_some() && !matches!(cmd, Command::Continue) {
        return Err(GameError::Finished);
    }
    match cmd {
        Command::Open(i) => {
            ensure_free(s)?;
            if i >= s.inbox.len() {
                return Err(GameError::Unknown("élément".into()));
            }
            let item = s.inbox.remove(i);
            s.active = Some(match item {
                Inbox::Event { id } => {
                    let def = content
                        .event(&id)
                        .ok_or(GameError::Unknown(id.clone()))?
                        .clone();
                    let now = s.turn();
                    s.seen.insert(id.clone(), now);
                    if !def.immediate.is_empty() {
                        effects::apply(s, &def.immediate);
                        s.check_game_over();
                    }
                    Active::Event { id }
                }
                Inbox::Renewal { contract } => Active::Renewal { contract },
                Inbox::Consequence {
                    title,
                    text,
                    effects: fx,
                    ..
                } => {
                    let deltas = effects::apply(s, &fx);
                    s.check_game_over();
                    Active::Result(Box::new(Resolution {
                        title: format!("{title} — retour de bâton"),
                        text,
                        deltas,
                        notes: vec![],
                        speaker: String::new(),
                        delayed_hint: false,
                    }))
                }
                Inbox::YearReview => Active::Review(Box::new(score::year_review(content, s))),
                Inbox::Chapter { index } => Active::Chapter { index },
            });
            Ok(())
        }
        Command::Choose(i) => {
            let active = s
                .active
                .clone()
                .ok_or(GameError::NotNow("Aucune décision en cours.".into()))?;
            match active {
                Active::Event { id } => {
                    let def = content
                        .event(&id)
                        .ok_or(GameError::Unknown(id.clone()))?
                        .clone();
                    let ch = def
                        .choices
                        .get(i)
                        .ok_or(GameError::Unknown("choix".into()))?
                        .clone();
                    if !ch.requires.met(s) {
                        return Err(GameError::NotNow(format!(
                            "Option verrouillée : {}",
                            ch.requires.explain(s, &|t| content.tech_name(t))
                        )));
                    }
                    if ch.agenda_cost > 0 {
                        spend_agenda(s, ch.agenda_cost)?;
                    }
                    let title = s.tx(&def.title);
                    s.active = resolve_choice(content, s, &title, &ch, &id);
                    Ok(())
                }
                Active::Renewal { contract } => {
                    let def = content
                        .contract(&contract)
                        .ok_or(GameError::Unknown(contract.clone()))?
                        .clone();
                    match i {
                        0 => {
                            let n = negotiation::start(content, s, &contract, false)
                                .ok_or(GameError::Unknown(contract.clone()))?;
                            s.active = Some(Active::Negotiation(Box::new(n)));
                        }
                        1 => {
                            let n = negotiation::start(content, s, &contract, false)
                                .ok_or(GameError::Unknown(contract.clone()))?;
                            let r = negotiation::conclude(content, s, &n, true);
                            s.active = Some(Active::Result(Box::new(r)));
                        }
                        k => {
                            let p = def
                                .migration
                                .get(k - 2)
                                .ok_or(GameError::Unknown("option".into()))?
                                .clone();
                            projects::can_start(content, s, &p).map_err(GameError::NotNow)?;
                            let mut deltas = projects::start(content, s, &p);
                            let now = s.turn();
                            if let Some(c) = s.contracts.iter_mut().find(|c| c.id == contract) {
                                c.monthly_cost *= 1.0 + def.increase / 2.0;
                                c.next_renewal = now + 12;
                                c.renewals += 1;
                            }
                            effects::merge(
                                &mut deltas,
                                effects::apply(
                                    s,
                                    &Effects::from([
                                        (Metric::Sovereignty, 2.0),
                                        (Metric::Comex, -2.0),
                                    ]),
                                ),
                            );
                            s.push_log(
                                format!("{} prolongé 12 mois ; migration lancée.", def.label),
                                Tone::Story,
                            );
                            s.decisions.push(format!("migrate:{contract}:{p}"));
                            let pname = content
                                .project(&p)
                                .map(|x| x.name.clone())
                                .unwrap_or_default();
                            s.active = Some(result(
                                &format!("Renouvellement — {}", def.label),
                                &format!(
                                    "Vous signez une prolongation d'un an « le temps de migrer ». Le commercial note la date dans son CRM. Projet lancé : {pname}."
                                ),
                                deltas,
                                vec![],
                                false,
                            ));
                        }
                    }
                    Ok(())
                }
                _ => Err(GameError::NotNow("Pas de choix ici.".into())),
            }
        }
        Command::Continue => {
            match s.active.clone() {
                Some(Active::Result(_)) | Some(Active::Chapter { .. }) => {
                    s.active = None;
                }
                Some(Active::Notice { .. }) => s.active = None,
                Some(Active::Review(r)) => {
                    s.active = None;
                    let year = r.year;
                    s.flags.insert(format!("reviewed_y{year}"));
                    s.metrics.add(Metric::Comex, r.comex_delta);
                    s.year_scores.push(r.score);
                    s.push_log(
                        format!(
                            "COMEX annuel année {year} : score {} — « {} ».",
                            r.score, r.rank
                        ),
                        Tone::Story,
                    );
                    s.check_game_over();
                    if s.over.is_none() {
                        if r.last {
                            s.ending = Some(score::ending(s));
                        } else {
                            // Nouvelle année : le run de référence est remis à jour.
                            s.baseline_run = score::run_cost(s);
                            advance(content, s);
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        }
        Command::AdvanceMonth => {
            ensure_free(s)?;
            if !s.inbox.is_empty() {
                return Err(GameError::NotNow(format!(
                    "{} sujet(s) à traiter avant de passer au mois suivant.",
                    s.inbox.len()
                )));
            }
            if s.cal.month == 12 && !s.has_flag(&format!("reviewed_y{}", s.cal.year)) {
                // Le mois de décembre se termine par le COMEX annuel.
                let evs = end_month(content, s);
                for e in evs {
                    s.inbox.push(Inbox::Event { id: e });
                }
                if s.over.is_none() {
                    s.inbox.push(Inbox::YearReview);
                }
                s.flags.insert(format!("december_closed_y{}", s.cal.year));
                return Ok(());
            }
            advance(content, s);
            Ok(())
        }
        Command::StartProject(id) => {
            ensure_free(s)?;
            projects::can_start(content, s, &id).map_err(GameError::NotNow)?;
            spend_agenda(s, 1)?;
            let deltas = projects::start(content, s, &id);
            let def = content
                .project(&id)
                .cloned()
                .ok_or(GameError::Unknown(id.clone()))?;
            s.active = Some(result(
                &format!("Lancement : {}", def.name),
                &format!(
                    "Kick-off organisé. {} mois prévus. Le planning est « ambitieux mais réaliste ».",
                    def.duration
                ),
                deltas,
                vec![],
                false,
            ));
            Ok(())
        }
        Command::EsnBoost(id) => {
            ensure_free(s)?;
            if s.m(Metric::Budget) < 3.0 {
                return Err(GameError::NotNow(
                    "Budget insuffisant pour un renfort.".into(),
                ));
            }
            let p = s
                .projects
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or(GameError::Unknown(id.clone()))?;
            if p.esn {
                return Err(GameError::NotNow("Renfort déjà en place.".into()));
            }
            p.esn = true;
            let deltas = effects::apply(
                s,
                &Effects::from([
                    (Metric::Budget, -3.0),
                    (Metric::VendorDep, 3.0),
                    (Metric::Make, -1.0),
                ]),
            );
            s.push_log(
                "Renfort ESN : « Nous avons exactement le profil. » Il arrive dans trois semaines.",
                Tone::Money,
            );
            s.active = Some(result(
                "Renfort ESN",
                "Le business manager vous envoie 4 CV. Trois sont la même personne. Le projet accélère, la connaissance part chez le prestataire.",
                deltas,
                vec![],
                false,
            ));
            Ok(())
        }
        Command::CancelProject(id) => {
            ensure_free(s)?;
            if !s.projects.iter().any(|p| p.id == id) {
                return Err(GameError::Unknown(id));
            }
            s.projects.retain(|p| p.id != id);
            let deltas = effects::apply(
                s,
                &Effects::from([(Metric::Comex, -4.0), (Metric::DevSat, -3.0)]),
            );
            let name = content
                .project(&id)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            s.push_log(
                format!("Projet abandonné : {name}. On parlera de « recentrage stratégique »."),
                Tone::Bad,
            );
            s.active = Some(result(
                "Projet abandonné",
                "Le budget déjà dépensé ne reviendra pas. Le slide « Lessons learned » est prêt.",
                deltas,
                vec![],
                false,
            ));
            Ok(())
        }
        Command::DoAction(id) => {
            ensure_free(s)?;
            let a = content
                .action(&id)
                .cloned()
                .ok_or(GameError::Unknown(id.clone()))?;
            if !a.requires.met(s) {
                return Err(GameError::NotNow(
                    a.requires.explain(s, &|t| content.tech_name(t)),
                ));
            }
            if let Some(last) = s.actions_done.get(&id)
                && a.cooldown > 0
                && s.turn() < last + a.cooldown
            {
                return Err(GameError::NotNow(format!(
                    "Déjà fait récemment (encore {} mois).",
                    last + a.cooldown - s.turn()
                )));
            }
            spend_agenda(s, a.agenda)?;
            let now = s.turn();
            s.actions_done.insert(id.clone(), now);
            let ch = a.as_choice();
            s.active = resolve_choice(content, s, &a.label, &ch, &id);
            Ok(())
        }
        Command::Negotiate(contract) => {
            ensure_free(s)?;
            let st = s
                .contracts
                .iter()
                .find(|c| c.id == contract && c.active)
                .ok_or(GameError::Unknown(contract.clone()))?;
            if st.pre_deal.is_some() {
                return Err(GameError::NotNow(
                    "Pré-accord déjà signé pour ce contrat.".into(),
                ));
            }
            if st.months_left(s.turn()) < 1 {
                return Err(GameError::NotNow(
                    "Échéance atteinte : le renouvellement est dans votre boîte.".into(),
                ));
            }
            if st.months_left(s.turn()) > 9 {
                return Err(GameError::NotNow(
                    "Trop tôt : le commercial n'a pas encore de budget à défendre.".into(),
                ));
            }
            spend_agenda(s, 1)?;
            let n = negotiation::start(content, s, &contract, true)
                .ok_or(GameError::Unknown(contract))?;
            s.active = Some(Active::Negotiation(Box::new(n)));
            Ok(())
        }
        Command::Dismiss(idx) => {
            ensure_free(s)?;
            if idx >= s.team.len() {
                return Err(GameError::Unknown(format!("recrue {idx}")));
            }
            spend_agenda(s, 1)?;
            let r = people::dismiss(content, s, idx)
                .ok_or(GameError::Unknown(format!("recrue {idx}")))?;
            s.active = Some(Active::Result(Box::new(r)));
            Ok(())
        }
        Command::Recruit(role) => {
            ensure_free(s)?;
            let r = content
                .careers
                .roles
                .iter()
                .find(|r| r.id == role)
                .ok_or(GameError::Unknown(role.clone()))?;
            if !r.requires.met(s) {
                return Err(GameError::NotNow(
                    r.requires.explain(s, &|t| content.tech_name(t)),
                ));
            }
            if s.team.len() >= 12 {
                return Err(GameError::NotNow(
                    "La DRH a gelé les embauches (« plafond d'emplois »).".into(),
                ));
            }
            spend_agenda(s, 1)?;
            let iv = people::start(content, s, &role).ok_or(GameError::Unknown(role))?;
            s.active = Some(Active::Interview(Box::new(iv)));
            Ok(())
        }
        Command::PlayCard(card) => {
            let Some(Active::Negotiation(mut n)) = s.active.clone() else {
                return Err(GameError::NotNow("Pas de négociation.".into()));
            };
            if !negotiation::play(content, s, &mut n, &card) {
                return Err(GameError::NotNow("Carte non jouable.".into()));
            }
            s.active = Some(Active::Negotiation(n));
            Ok(())
        }
        Command::Conclude | Command::AcceptOffer => {
            let Some(Active::Negotiation(n)) = s.active.clone() else {
                return Err(GameError::NotNow("Pas de négociation.".into()));
            };
            let r = negotiation::conclude(content, s, &n, matches!(cmd, Command::AcceptOffer));
            s.active = Some(Active::Result(Box::new(r)));
            Ok(())
        }
        Command::Answer(i) => {
            let Some(Active::Interview(mut iv)) = s.active.clone() else {
                return Err(GameError::NotNow("Pas d'entretien.".into()));
            };
            if !people::answer(content, s, &mut iv, i) {
                return Err(GameError::Unknown("réponse".into()));
            }
            s.active = Some(Active::Interview(iv));
            Ok(())
        }
        Command::MakeOffer(i) => {
            let Some(Active::Interview(iv)) = s.active.clone() else {
                return Err(GameError::NotNow("Pas d'entretien.".into()));
            };
            let r = people::offer(content, s, &iv, i);
            s.active = Some(Active::Result(Box::new(r)));
            s.check_game_over();
            Ok(())
        }
        Command::War(a) => {
            let Some(Active::WarRoom(mut w)) = s.active.clone() else {
                return Err(GameError::NotNow("Pas de crise.".into()));
            };
            security::act(content, s, &mut w, a);
            s.active = Some(Active::WarRoom(w));
            Ok(())
        }
        Command::WarClose => {
            let Some(Active::WarRoom(w)) = s.active.clone() else {
                return Err(GameError::NotNow("Pas de crise.".into()));
            };
            let r = security::close(content, s, &w);
            s.active = Some(Active::Result(Box::new(r)));
            Ok(())
        }
        Command::DebugSet(m, v) => {
            s.metrics.set(m, v);
            Ok(())
        }
        Command::DebugEvent(id) => {
            if content.event(&id).is_none() {
                return Err(GameError::Unknown(id));
            }
            s.inbox.insert(0, Inbox::Event { id });
            Ok(())
        }
        Command::DebugTech(id) => {
            s.techs.insert(id);
            Ok(())
        }
        Command::DebugAgenda => {
            s.agenda = s.agenda_max;
            Ok(())
        }
        Command::ConsultMentor(id) => {
            ensure_free(s)?;
            let r = crate::mentors::consult(content, s, &id).map_err(GameError::NotNow)?;
            s.active = Some(Active::Result(Box::new(r)));
            Ok(())
        }
        Command::ExtendCampaign => {
            let Some(ending) = s.ending.take() else {
                return Err(GameError::NotNow("La partie n'est pas terminée.".into()));
            };
            extend(content, s, ending);
            Ok(())
        }
        Command::SetAmbition(id) => {
            if s.extension == 0 {
                return Err(GameError::NotNow(
                    "Le cap se choisit au début de la saison 2.".into(),
                ));
            }
            if let Some(id) = &id
                && !score::ending_goals(s).iter().any(|g| g.id == id)
            {
                return Err(GameError::Unknown(id.clone()));
            }
            if s.ambition == id {
                return Ok(());
            }
            let title = |id: &Option<String>, s: &GameState| {
                id.as_deref()
                    .and_then(|a| score::ending_goals(s).into_iter().find(|g| g.id == a))
                    .map_or("aucun cap".to_string(), |g| format!("« {} »", g.title))
            };
            if s.ambition.is_some() {
                if s.agenda == 0 {
                    return Err(GameError::NoAgenda);
                }
                s.agenda -= 1;
                s.metrics.add(Metric::Comex, -2.0);
                s.push_log(
                    format!(
                        "Changement de cap : {} → {}. Le COMEX note le revirement (COMEX −2).",
                        title(&s.ambition, s),
                        title(&id, s)
                    ),
                    Tone::Story,
                );
            } else {
                s.push_log(
                    format!(
                        "Cap de la saison 2 : {}. Atteint en fin d'année 6 : +{} points de score.",
                        title(&id, s),
                        score::AMBITION_BONUS
                    ),
                    Tone::Story,
                );
            }
            s.decisions
                .push(format!("ambition:{}", id.as_deref().unwrap_or("none")));
            s.ambition = id;
            Ok(())
        }
        Command::SetTutorial(step) => {
            s.tutorial = step;
            Ok(())
        }
        Command::DebugHire(role) => {
            let r = content
                .careers
                .roles
                .iter()
                .find(|r| r.id == role)
                .ok_or(GameError::Unknown(role.clone()))?;
            let names = &content.careers.first_names;
            let name = format!(
                "{} {}",
                names[s.team.len() % names.len()],
                content.careers.last_names[s.team.len() % content.careers.last_names.len()]
            );
            let since = s.turn();
            s.team.push(people::Hire {
                role: role.clone(),
                name,
                salary: r.salary,
                since,
            });
            Ok(())
        }
        Command::DebugSkip => {
            s.inbox.clear();
            s.active = None;
            if s.cal.month == 12 {
                let r = score::year_review(content, s);
                s.active = Some(Active::Review(Box::new(r)));
            } else {
                advance(content, s);
            }
            Ok(())
        }
    }
}

/// Prolonge la partie de quelques années après une fin de campagne.
fn extend(content: &Content, s: &mut GameState, ending: crate::state::Ending) {
    let years = content.dialogues.extension_years;
    s.push_log(
        format!(
            "Fin « {} » atteinte en année {} (score {}). Le conseil vous prolonge de {years} ans.",
            ending.title, s.cal.year, ending.score
        ),
        Tone::Story,
    );
    s.past_endings.push(ending);
    s.extension += years;
    s.decisions.push(format!("extend:{}", s.extension));
    // Les vieux problèmes reviennent : les événements aléatoires vus il y a plus de 2 ans
    // peuvent se reproduire.
    let now = s.turn();
    let random: std::collections::BTreeSet<&str> = content
        .events
        .iter()
        .filter(|e| e.at.is_none() && e.weight > 0.0)
        .map(|e| e.id.as_str())
        .collect();
    s.seen
        .retain(|id, t| !(random.contains(id.as_str()) && now >= *t + 24));
    s.baseline_run = score::run_cost(s);
    advance(content, s);
}

/// Termine le mois courant et commence le suivant.
fn advance(content: &Content, s: &mut GameState) {
    let closed_already = s.has_flag(&format!("december_closed_y{}", s.cal.year));
    let evs = if closed_already {
        Vec::new()
    } else {
        end_month(content, s)
    };
    if s.over.is_some() {
        return;
    }
    s.cal = Calendar::from_abs(s.turn() + 1);
    begin_month(content, s);
    for e in evs {
        if !s
            .inbox
            .iter()
            .any(|i| matches!(i, Inbox::Event { id } if *id == e))
        {
            s.inbox.push(Inbox::Event { id: e });
        }
    }
}

/// Choix d'une action « automatique » raisonnable : utilisé par l'autopilote des tests.
pub fn autopilot_choice(content: &Content, s: &GameState) -> usize {
    match &s.active {
        Some(Active::Event { id }) => content
            .event(id)
            .map(|e| {
                e.choices
                    .iter()
                    .position(|c| c.requires.met(s) && s.agenda >= c.agenda_cost)
                    .unwrap_or(0)
            })
            .unwrap_or(0),
        _ => 1,
    }
}
