//! Équilibrage : joue des centaines de parties avec plusieurs stratégies et affiche
//! la répartition des fins, des défaites et des métriques finales.
//!
//!   cargo run -p dsi-core --example balance --release

use dsi_core::content::Content;
use dsi_core::game::{self, Command};
use dsi_core::security::WarAction;
use dsi_core::state::{Active, GameState, Metric, Profile};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
enum Strat {
    Random,
    Greedy,
    Passive,
}

fn value(s: &GameState, fx: &dsi_core::effects::Effects) -> f32 {
    let mut v = 0.0;
    for (m, d) in fx {
        let w = match m {
            Metric::Comex | Metric::Budget | Metric::Security => 1.5,
            Metric::Debt | Metric::VendorDep | Metric::Complexity => -1.0,
            _ => 0.6,
        };
        // Plus une métrique est proche d'un seuil de défaite, plus elle compte.
        let danger = match m {
            Metric::Debt | Metric::VendorDep | Metric::Complexity => {
                (s.m(*m) / 100.0).powi(2) * 3.0
            }
            Metric::Comex | Metric::Budget | Metric::Security | Metric::Make => {
                ((100.0 - s.m(*m)) / 100.0).powi(2) * 3.0
            }
            _ => 0.0,
        };
        v += d * w * (1.0 + danger);
    }
    v
}

fn pick(
    _c: &Content,
    s: &GameState,
    n: usize,
    strat: Strat,
    rng: &mut ChaCha8Rng,
    choices: &[dsi_core::effects::ChoiceDef],
) -> usize {
    match strat {
        Strat::Random => rng.random_range(0..n),
        Strat::Passive => n - 1,
        Strat::Greedy => (0..n)
            .filter(|i| choices[*i].requires.met(s))
            .max_by(|a, b| {
                let va = value(s, &choices[*a].outcome.effects)
                    + if choices[*a].outcome.start_project.is_some() {
                        3.0
                    } else {
                        0.0
                    };
                let vb = value(s, &choices[*b].outcome.effects)
                    + if choices[*b].outcome.start_project.is_some() {
                        3.0
                    } else {
                        0.0
                    };
                va.total_cmp(&vb)
            })
            .unwrap_or(0),
    }
    .min(n - 1)
}

fn resolve(c: &Content, s: &mut GameState, strat: Strat, rng: &mut ChaCha8Rng) {
    for _ in 0..60 {
        let Some(a) = s.active.clone() else { return };
        let r = match a {
            Active::Event { id } => {
                let def = c.event(&id).unwrap().clone();
                let n = def.choices.len();
                let first = pick(c, s, n, strat, rng, &def.choices);
                let mut ok = Err(game::GameError::Busy);
                for k in 0..n {
                    ok = game::apply(c, s, Command::Choose((first + k) % n));
                    if ok.is_ok() {
                        break;
                    }
                }
                ok
            }
            Active::Renewal { .. } => game::apply(
                c,
                s,
                Command::Choose(if matches!(strat, Strat::Passive) {
                    1
                } else {
                    0
                }),
            ),
            Active::Negotiation(n) => {
                let card = n
                    .hand
                    .iter()
                    .find(|h| !n.played.contains(h) && !h.contains("5ans"))
                    .cloned();
                match (n.rounds_left > 0, card) {
                    (true, Some(card)) => game::apply(c, s, Command::PlayCard(card))
                        .or_else(|_| game::apply(c, s, Command::Conclude)),
                    _ => game::apply(c, s, Command::Conclude),
                }
            }
            Active::Interview(iv) => {
                if iv.gone || iv.step >= iv.questions.len() {
                    game::apply(c, s, Command::MakeOffer(Some(1)))
                } else {
                    let q = dsi_core::people::current_question(c, &iv).unwrap();
                    let i = rng.random_range(0..q.answers.len());
                    game::apply(c, s, Command::Answer(i))
                }
            }
            Active::WarRoom(w) => {
                if dsi_core::security::done(&w) {
                    game::apply(c, s, Command::WarClose)
                } else {
                    let a = match strat {
                        Strat::Random => WarAction::ALL[rng.random_range(0..7)],
                        _ => [
                            WarAction::Isolate,
                            WarAction::InformComex,
                            WarAction::Investigate,
                            WarAction::Patch,
                            WarAction::Restore,
                            WarAction::Communicate,
                        ][(6 - w.hours_left as usize).min(5)],
                    };
                    game::apply(c, s, Command::War(a))
                }
            }
            _ => game::apply(c, s, Command::Continue),
        };
        r.unwrap();
    }
}

fn play(c: &Content, seed: u64, strat: Strat) -> GameState {
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xABCD);
    let mut s = game::new_game(c, seed, Profile::default());
    play_on(c, &mut s, seed, strat, &mut rng);
    s
}

fn play_on(c: &Content, s: &mut GameState, seed: u64, strat: Strat, rng: &mut ChaCha8Rng) {
    for _ in 0..600 {
        if s.finished() {
            break;
        }
        if s.active.is_some() {
            resolve(c, s, strat, rng);
            continue;
        }
        if !s.inbox.is_empty() {
            game::apply(c, s, Command::Open(0)).unwrap();
            continue;
        }
        if s.agenda > 0 && !matches!(strat, Strat::Passive) {
            let projects: Vec<String> = c
                .projects
                .iter()
                .filter(|p| p.catalog && dsi_core::projects::can_start(c, s, &p.id).is_ok())
                .map(|p| p.id.clone())
                .collect();
            if !projects.is_empty() && s.m(Metric::Budget) > 30.0 && rng.random::<f32>() < 0.6 {
                let p = projects[rng.random_range(0..projects.len())].clone();
                game::apply(c, s, Command::StartProject(p)).unwrap();
                continue;
            }
            let acts: Vec<String> = c
                .actions
                .iter()
                .filter(|a| {
                    a.requires.met(s)
                        && s.actions_done
                            .get(&a.id)
                            .is_none_or(|t| s.turn() >= t + a.cooldown)
                })
                .map(|a| a.id.clone())
                .collect();
            if !acts.is_empty() {
                let a = acts[rng.random_range(0..acts.len())].clone();
                if game::apply(c, s, Command::DoAction(a)).is_ok() {
                    continue;
                }
            }
        }
        if std::env::var("TRACE").is_ok() && seed == 1 {
            trace(c, s);
        }
        game::apply(c, s, Command::AdvanceMonth).unwrap();
    }
}

fn main() {
    let c = Content::load().unwrap();
    // SAVE_END=fichier : écrit la sauvegarde d'une partie arrivée à sa fin (tests du client).
    if let Ok(path) = std::env::var("SAVE_END") {
        for seed in 0..100 {
            let s = play(&c, seed, Strat::Greedy);
            if let Some(e) = &s.ending {
                std::fs::write(&path, dsi_core::save::to_json(&s).unwrap()).unwrap();
                println!("graine {seed} : {}", e.title);
                return;
            }
        }
    }
    season_two(&c);
    let n = 300;
    for strat in [Strat::Random, Strat::Greedy, Strat::Passive] {
        let mut outcomes: BTreeMap<String, u32> = BTreeMap::new();
        let mut months = Vec::new();
        let mut avg: BTreeMap<Metric, f32> = BTreeMap::new();
        let mut scores = Vec::new();
        let mut y1 = Vec::new();
        for seed in 0..n {
            let s = play(&c, seed, strat);
            let key = if let Some(o) = s.over {
                format!("DÉFAITE {}", o.title())
            } else if let Some(e) = &s.ending {
                format!("FIN {}", e.title)
            } else {
                "en cours".into()
            };
            *outcomes.entry(key).or_default() += 1;
            months.push(s.turn());
            for m in dsi_core::state::Metric::ALL {
                *avg.entry(m).or_default() += s.m(m) / n as f32;
            }
            scores.push(dsi_core::score::score(&s));
            if let Some(v) = s.year_scores.first() {
                y1.push(*v);
            }
        }
        println!("\n=== {strat:?} ({n} parties) ===");
        for (k, v) in &outcomes {
            println!("  {v:>4}  {k}");
        }
        months.sort();
        scores.sort();
        println!(
            "  mois atteint : médiane {} (min {}, max {})",
            months[months.len() / 2],
            months[0],
            months[months.len() - 1]
        );
        println!(
            "  score final : médiane {} (p10 {}, p90 {})",
            scores[scores.len() / 2],
            scores[scores.len() / 10],
            scores[scores.len() * 9 / 10]
        );
        if !y1.is_empty() {
            y1.sort();
            println!("  score année 1 : médiane {}", y1[y1.len() / 2]);
        }
        let line: Vec<String> = avg
            .iter()
            .map(|(m, v)| format!("{}={:.0}", m.key(), v))
            .collect();
        println!("  moyennes finales : {}", line.join(" "));
    }
}

#[allow(dead_code)]
pub fn trace(c: &Content, s: &GameState) {
    println!(
        "  {:>2}/{} budget={:>5.1} alloc={:.2} contrats={:.2} paie={:.2} cloud={:.2} burn={:.2} projets={} equipe={}",
        s.cal.month,
        s.cal.year,
        s.m(Metric::Budget),
        s.allocation,
        dsi_core::contracts::run_cost(s),
        dsi_core::people::payroll(s),
        s.cloud.bill,
        dsi_core::projects::burn(c, s),
        s.projects.len(),
        s.team.len()
    );
}

/// Saison 2 : prolonge les parties terminées et compare la fin de l'année 3 à celle de l'année 6.
fn season_two(c: &Content) {
    for strat in [Strat::Random, Strat::Greedy] {
        let mut outcomes: BTreeMap<String, u32> = BTreeMap::new();
        let mut before = Vec::new();
        let mut after = Vec::new();
        for seed in 0..300 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xABCD);
            let mut s = game::new_game(c, seed, Profile::default());
            play_on(c, &mut s, seed, strat, &mut rng);
            let Some(e) = s.ending.clone() else { continue };
            before.push(e.score);
            game::apply(c, &mut s, Command::ExtendCampaign).unwrap();
            play_on(c, &mut s, seed, strat, &mut rng);
            let key = if let Some(o) = s.over {
                format!("DÉFAITE {} (année {})", o.title(), s.cal.year)
            } else if let Some(e) = &s.ending {
                after.push(e.score);
                format!("FIN {}", e.title)
            } else {
                format!("EN COURS {:?}", s.cal)
            };
            *outcomes.entry(key).or_default() += 1;
        }
        let med = |v: &mut Vec<u32>| {
            v.sort();
            v.get(v.len() / 2).copied().unwrap_or(0)
        };
        println!(
            "\n=== Saison 2 — {strat:?} ({} parties prolongées) ===",
            before.len()
        );
        let mut v: Vec<_> = outcomes.into_iter().collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        for (k, n) in v {
            println!("  {n:>4}  {k}");
        }
        println!(
            "  score année 3 : médiane {} · score année 6 : médiane {}",
            med(&mut before),
            med(&mut after)
        );
    }
}
