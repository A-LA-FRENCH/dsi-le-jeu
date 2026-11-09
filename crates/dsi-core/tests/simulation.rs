//! Tests de la simulation : contenu, règles économiques, événements, contrats,
//! négociation, recrutement, war room, sauvegarde, score et campagne complète.

use dsi_core::content::Content;
use dsi_core::effects::Minigame;
use dsi_core::game::{self, Command};
use dsi_core::security::WarAction;
use dsi_core::state::{Active, GameState, Inbox, Metric, Profile};
use dsi_core::{save, score};

fn content() -> Content {
    Content::load().expect("les données doivent se charger")
}

fn new(seed: u64) -> (Content, GameState) {
    let c = content();
    let s = game::new_game(&c, seed, Profile::default());
    (c, s)
}

/// Joue une scène active avec une stratégie simple (index de choix `pick`).
fn resolve_active(c: &Content, s: &mut GameState, pick: usize) {
    for _ in 0..40 {
        let Some(a) = s.active.clone() else { return };
        let r = match a {
            Active::Event { id } => {
                let def = c.event(&id).unwrap();
                let n = def.choices.len();
                let mut ok = Err(game::GameError::Busy);
                for k in 0..n {
                    ok = game::apply(c, s, Command::Choose((pick + k) % n));
                    if ok.is_ok() {
                        break;
                    }
                }
                ok
            }
            Active::Renewal { .. } => game::apply(c, s, Command::Choose(pick % 2)),
            Active::Negotiation(n) => {
                if n.rounds_left > 0 && !n.hand.is_empty() {
                    let card = n.hand.iter().find(|h| !n.played.contains(h)).cloned();
                    match card {
                        Some(card) => game::apply(c, s, Command::PlayCard(card))
                            .or_else(|_| game::apply(c, s, Command::Conclude)),
                        None => game::apply(c, s, Command::Conclude),
                    }
                } else {
                    game::apply(c, s, Command::Conclude)
                }
            }
            Active::Interview(iv) => {
                if iv.gone || iv.step >= iv.questions.len() {
                    game::apply(c, s, Command::MakeOffer(Some(1)))
                } else {
                    let n = dsi_core::people::current_question(c, &iv)
                        .map(|q| q.answers.len())
                        .unwrap_or(1);
                    game::apply(c, s, Command::Answer((pick % 2 + 1).min(n - 1)))
                }
            }
            Active::WarRoom(w) => {
                if dsi_core::security::done(&w) {
                    game::apply(c, s, Command::WarClose)
                } else {
                    let order = [
                        WarAction::InformComex,
                        WarAction::Isolate,
                        WarAction::Investigate,
                        WarAction::Patch,
                        WarAction::Restore,
                        WarAction::Communicate,
                    ];
                    let i = 6 - w.hours_left as usize;
                    game::apply(c, s, Command::War(order[i.min(5)]))
                }
            }
            _ => game::apply(c, s, Command::Continue),
        };
        r.expect("commande valide");
    }
}

/// Joue une partie complète ; renvoie l'état final.
fn play(seed: u64, pick: usize) -> GameState {
    let (c, mut s) = new(seed);
    play_on(&c, &mut s, pick);
    s
}

/// Fait avancer une partie existante jusqu'à sa fin (ou 1 500 commandes).
fn play_on(c: &Content, s: &mut GameState, pick: usize) {
    let (c, s) = (c, s);
    for _ in 0..1500 {
        if s.finished() {
            break;
        }
        if s.active.is_some() {
            resolve_active(c, s, pick);
            continue;
        }
        if !s.inbox.is_empty() {
            game::apply(c, s, Command::Open(0)).unwrap();
            continue;
        }
        // Utilise l'agenda : un projet si possible, sinon une action.
        if s.agenda > 0 {
            let p = c
                .projects
                .iter()
                .find(|p| p.catalog && dsi_core::projects::can_start(c, s, &p.id).is_ok())
                .map(|p| p.id.clone());
            if let Some(p) = p
                && s.m(Metric::Budget) > 25.0
            {
                game::apply(c, s, Command::StartProject(p)).unwrap();
                continue;
            }
            let a = c
                .actions
                .iter()
                .find(|a| {
                    a.requires.met(s)
                        && s.actions_done
                            .get(&a.id)
                            .is_none_or(|t| s.turn() >= t + a.cooldown.max(1))
                })
                .map(|a| a.id.clone());
            if let Some(a) = a
                && game::apply(c, s, Command::DoAction(a)).is_ok()
            {
                continue;
            }
        }
        game::apply(c, s, Command::AdvanceMonth).unwrap();
    }
}

#[test]
fn le_contenu_est_valide() {
    let c = content();
    let errs = c.validate();
    assert!(errs.is_empty(), "erreurs de contenu :\n{}", errs.join("\n"));
    assert!(
        c.events.len() >= 60,
        "au moins 60 événements ({})",
        c.events.len()
    );
    assert!(c.vendors.len() >= 5);
    assert!(c.techs.len() >= 5);
    assert!(c.projects.len() >= 3);
}

#[test]
fn la_campagne_annee_1_suit_le_scenario() {
    let c = content();
    for (m, id) in [
        (1, "dg_welcome"),
        (2, "audit_si"),
        (3, "vmware_price"),
        (4, "m365_meeting"),
        (5, "cve_mai"),
        (6, "architect_leaves"),
        (7, "budget_cut"),
        (8, "datacenter_clim"),
        (9, "cloud_proposal"),
        (10, "acquisition"),
        (11, "erp_wars"),
        (12, "comex_annuel_1"),
    ] {
        let e = c.event(id).unwrap_or_else(|| panic!("{id} manquant"));
        let at = e.at.expect("scripté");
        assert_eq!((at.year, at.month), (1, m), "{id}");
    }
}

#[test]
fn une_nouvelle_partie_commence_en_janvier_avec_la_dg() {
    let (_, s) = new(1);
    assert_eq!((s.cal.year, s.cal.month), (1, 1));
    assert!(
        s.inbox
            .iter()
            .any(|i| matches!(i, Inbox::Event { id } if id == "dg_welcome"))
    );
    assert!(s.has_tech("vmware") && s.has_tech("cobol"));
    assert!(
        s.contracts
            .iter()
            .any(|c| c.id == "vmware_vsphere" && c.active)
    );
    assert!(s.baseline_run > 5.0);
}

#[test]
fn on_ne_peut_pas_passer_le_mois_avec_des_sujets_en_attente() {
    let (c, mut s) = new(2);
    assert!(game::apply(&c, &mut s, Command::AdvanceMonth).is_err());
}

#[test]
fn les_choix_ont_des_effets_et_les_consequences_sont_differees() {
    let (c, mut s) = new(3);
    let i = s
        .inbox
        .iter()
        .position(|x| matches!(x, Inbox::Event { id } if id == "dg_welcome"))
        .unwrap();
    game::apply(&c, &mut s, Command::Open(i)).unwrap();
    let comex = s.m(Metric::Comex);
    game::apply(&c, &mut s, Command::Choose(0)).unwrap(); // « Promettre les deux »
    assert!(s.m(Metric::Comex) > comex);
    assert!(
        s.scheduled.iter().any(|x| x.due == 6),
        "conséquence programmée en juillet"
    );
    assert!(matches!(s.active, Some(Active::Result(_))));
}

#[test]
fn le_temps_avance_et_l_economie_tourne() {
    let (c, mut s) = new(4);
    s.inbox.clear();
    let debt = s.m(Metric::Debt);
    game::apply(&c, &mut s, Command::AdvanceMonth).unwrap();
    assert_eq!(s.cal.month, 2);
    assert!(s.m(Metric::Debt) > debt, "la dette monte toute seule");
    assert_eq!(s.agenda, s.agenda_max);
}

#[test]
fn le_cout_de_fonctionnement_pese_sur_le_budget() {
    let (c, mut s) = new(5);
    let before = dsi_core::economy::monthly_balance(&c, &s);
    s.contracts.iter_mut().for_each(|x| x.monthly_cost *= 2.0);
    let after = dsi_core::economy::monthly_balance(&c, &s);
    assert!(after < before);
}

#[test]
fn les_projets_avancent_et_se_livrent() {
    let (c, mut s) = new(6);
    s.inbox.clear();
    game::apply(&c, &mut s, Command::StartProject("forge".into())).unwrap();
    game::apply(&c, &mut s, Command::Continue).unwrap();
    assert_eq!(s.agenda, s.agenda_max - 1);
    for _ in 0..8 {
        s.inbox.clear();
        s.active = None;
        game::apply(&c, &mut s, Command::AdvanceMonth).unwrap();
        if s.done_projects.contains("forge") {
            break;
        }
    }
    assert!(s.done_projects.contains("forge"));
    assert!(s.has_tech("git") && !s.has_tech("svn"));
}

#[test]
fn cinq_projets_maximum() {
    let (c, mut s) = new(7);
    s.inbox.clear();
    s.metrics.set(Metric::Budget, 100.0);
    s.agenda = 9;
    for p in ["forge", "pra", "cartographie", "sso_mfa", "data_governance"] {
        game::apply(&c, &mut s, Command::StartProject(p.into())).unwrap();
        game::apply(&c, &mut s, Command::Continue).unwrap();
    }
    assert!(game::apply(&c, &mut s, Command::StartProject("api_platform".into())).is_err());
}

#[test]
fn negociation_anticipee_donne_un_pre_accord() {
    let (c, mut s) = new(8);
    s.inbox.clear();
    // Mars : VMware renouvelle en juin.
    s.cal = dsi_core::state::Calendar { year: 1, month: 3 };
    game::apply(&c, &mut s, Command::Negotiate("vmware_vsphere".into())).unwrap();
    let Some(Active::Negotiation(n)) = s.active.clone() else {
        panic!()
    };
    assert!(n.early && n.months_left == 3);
    let lev = n.leverage;
    game::apply(&c, &mut s, Command::PlayCard("achats".into())).unwrap();
    let Some(Active::Negotiation(n2)) = s.active.clone() else {
        panic!()
    };
    assert!(n2.leverage >= lev - 30.0);
    game::apply(&c, &mut s, Command::Conclude).unwrap();
    let ct = s
        .contracts
        .iter()
        .find(|x| x.id == "vmware_vsphere")
        .unwrap();
    assert!(ct.pre_deal.is_some());
}

#[test]
fn attendre_j2_coute_cher() {
    let (c, mut s) = new(9);
    s.cal = dsi_core::state::Calendar { year: 1, month: 6 };
    let early = dsi_core::negotiation::start(&c, &mut s.clone(), "vmware_vsphere", true).unwrap();
    s.cal = dsi_core::state::Calendar { year: 1, month: 3 };
    let early3 = dsi_core::negotiation::start(&c, &mut s.clone(), "vmware_vsphere", true).unwrap();
    assert!(
        early3.leverage > early.leverage + 30.0,
        "{} vs {}",
        early3.leverage,
        early.leverage
    );
}

#[test]
fn le_renouvellement_arrive_a_echeance() {
    let (c, mut s) = new(10);
    for _ in 0..6 {
        s.inbox.retain(|i| {
            !matches!(
                i,
                Inbox::Event { .. } | Inbox::Chapter { .. } | Inbox::Consequence { .. }
            )
        });
        if s.inbox
            .iter()
            .any(|i| matches!(i, Inbox::Renewal { contract } if contract == "vmware_vsphere"))
        {
            break;
        }
        s.inbox.clear();
        s.active = None;
        game::apply(&c, &mut s, Command::AdvanceMonth).unwrap();
    }
    assert!(
        s.inbox
            .iter()
            .any(|i| matches!(i, Inbox::Renewal { contract } if contract == "vmware_vsphere"))
    );
    let idx = s
        .inbox
        .iter()
        .position(|i| matches!(i, Inbox::Renewal { .. }))
        .unwrap();
    let cost = s
        .contracts
        .iter()
        .find(|x| x.id == "vmware_vsphere")
        .unwrap()
        .monthly_cost;
    game::apply(&c, &mut s, Command::Open(idx)).unwrap();
    game::apply(&c, &mut s, Command::Choose(1)).unwrap(); // accepter
    let after = s
        .contracts
        .iter()
        .find(|x| x.id == "vmware_vsphere")
        .unwrap();
    assert!(after.monthly_cost > cost * 1.5);
    assert!(after.next_renewal > s.turn());
}

#[test]
fn recrutement_java_8_fait_fuir_le_candidat() {
    let (c, mut s) = new(11);
    let mut iv = dsi_core::people::start(&c, &mut s, "senior").unwrap();
    iv.questions = vec!["java".into(), "remote".into(), "stack".into()];
    dsi_core::people::answer(&c, &mut s, &mut iv, 0);
    assert!(iv.gone);
    let r = dsi_core::people::offer(&c, &mut s, &iv, Some(2));
    assert!(r.title.contains("échec"));
    assert!(s.team.is_empty());
}

#[test]
fn recrutement_genereux_finit_par_embaucher() {
    let c = content();
    let mut hired = 0;
    for seed in 0..20 {
        let mut s = game::new_game(&c, seed, Profile::default());
        s.metrics.set(Metric::Attractiveness, 70.0);
        let mut iv = dsi_core::people::start(&c, &mut s, "junior").unwrap();
        while iv.step < iv.questions.len() && !iv.gone {
            let q = dsi_core::people::current_question(&c, &iv).unwrap();
            let best = (0..q.answers.len())
                .max_by(|a, b| q.answers[*a].interest.total_cmp(&q.answers[*b].interest))
                .unwrap();
            dsi_core::people::answer(&c, &mut s, &mut iv, best);
        }
        dsi_core::people::offer(&c, &mut s, &iv, Some(2));
        hired += s.team.len();
    }
    assert!(hired >= 10, "{hired}/20");
}

#[test]
fn war_room_bien_menee_vaut_mieux_que_le_chaos() {
    let c = content();
    let mut good = game::new_game(&c, 12, Profile::default());
    let mut bad = good.clone();
    let mut w = dsi_core::security::start(&c, &good, "ransomware").unwrap();
    for a in [
        WarAction::InformComex,
        WarAction::Isolate,
        WarAction::Investigate,
        WarAction::Patch,
        WarAction::Restore,
        WarAction::Communicate,
    ] {
        dsi_core::security::act(&c, &mut good, &mut w, a);
    }
    dsi_core::security::close(&c, &mut good, &w);
    let mut w2 = dsi_core::security::start(&c, &bad, "ransomware").unwrap();
    for _ in 0..6 {
        dsi_core::security::act(&c, &mut bad, &mut w2, WarAction::Restore);
    }
    dsi_core::security::close(&c, &mut bad, &w2);
    assert!(good.m(Metric::Comex) > bad.m(Metric::Comex));
    assert!(good.m(Metric::Security) > bad.m(Metric::Security));
}

#[test]
fn sauvegarde_aller_retour() {
    let (c, mut s) = new(13);
    game::apply(&c, &mut s, Command::Open(0)).unwrap();
    let json = save::to_json(&s).unwrap();
    let back = save::from_json(&json).unwrap();
    assert_eq!(back, s);
    assert!(save::from_json("{}").is_err());
}

#[test]
fn meme_graine_meme_partie() {
    let a = play(42, 0);
    let b = play(42, 0);
    assert_eq!(save::to_json(&a).unwrap(), save::to_json(&b).unwrap());
}

#[test]
fn score_et_classement() {
    let (_, mut s) = new(14);
    for m in Metric::ALL {
        s.metrics.set(m, if m.inverted() { 0.0 } else { 100.0 });
    }
    assert_eq!(score::score(&s), 100);
    assert_eq!(score::rank(97), "Vous avez probablement triché.");
    assert_eq!(score::rank(10), "Directeur du PowerPoint");
    assert_eq!(score::ending(&s).id, "pragmatic");
}

#[test]
fn game_over_quand_le_comex_lache() {
    let (c, mut s) = new(15);
    game::apply(&c, &mut s, Command::DebugSet(Metric::Comex, 0.0)).unwrap();
    s.check_game_over();
    assert_eq!(s.over, Some(dsi_core::state::GameOver::Fired));
    assert!(game::apply(&c, &mut s, Command::AdvanceMonth).is_err());
}

#[test]
fn kubernetes_sans_competences_augmente_la_complexite() {
    let (c, mut s) = new(16);
    s.techs.insert("kubernetes".into());
    s.metrics.set(Metric::DevopsMaturity, 10.0);
    let cx = s.m(Metric::Complexity);
    dsi_core::technology::monthly(&c, &mut s);
    assert!(s.m(Metric::Complexity) > cx + 0.5);
    let mut s2 = s.clone();
    s2.metrics.set(Metric::DevopsMaturity, 70.0);
    s2.metrics.set(Metric::Make, 60.0);
    let v = s2.m(Metric::Velocity);
    dsi_core::technology::monthly(&c, &mut s2);
    assert!(s2.m(Metric::Velocity) > v);
}

#[test]
fn les_mini_jeux_sont_branches() {
    let c = content();
    let mut kinds = std::collections::BTreeSet::new();
    for e in &c.events {
        for ch in &e.choices {
            if let Some(m) = &ch.outcome.minigame {
                kinds.insert(match m {
                    Minigame::Negotiation { .. } => "nego",
                    Minigame::Recruit { .. } => "recrue",
                    Minigame::WarRoom { .. } => "warroom",
                });
            }
        }
    }
    assert_eq!(kinds.len(), 3);
}

#[test]
fn des_parties_completes_se_terminent() {
    let mut endings = 0;
    let mut overs = 0;
    for seed in 0..12 {
        let s = play(seed, (seed % 3) as usize);
        assert!(
            s.finished() || s.cal.year >= 3,
            "la partie {seed} n'avance pas : {:?} active={:?} inbox={:?} agenda={}",
            s.cal,
            s.active
                .as_ref()
                .map(|a| format!("{a:?}").chars().take(300).collect::<String>()),
            s.inbox,
            s.agenda
        );
        if s.ending.is_some() {
            endings += 1;
        }
        if s.over.is_some() {
            overs += 1;
        }
    }
    assert!(endings + overs >= 10, "fins {endings}, défaites {overs}");
}

#[test]
fn le_mentor_donne_un_bonus_permanent_et_des_conseils() {
    let c = content();
    let base = game::new_game(
        &c,
        20,
        Profile {
            mentor: String::new(),
            ..Profile::default()
        },
    );
    let s = game::new_game(
        &c,
        20,
        Profile {
            mentor: "jb".into(),
            ..Profile::default()
        },
    );
    assert!(s.m(Metric::Sovereignty) > base.m(Metric::Sovereignty));
    let mut s = s;
    s.inbox.clear();
    game::apply(&c, &mut s, Command::ConsultMentor("jb".into())).unwrap();
    let Some(Active::Result(r)) = s.active.clone() else {
        panic!()
    };
    assert_eq!(r.speaker, "mentor:jb");
    // Le conseil VMware de Jean-Baptiste donne du levier pour la prochaine négociation.
    assert!(s.has_flag("mentor_leverage"));
    game::apply(&c, &mut s, Command::Continue).unwrap();
    // Délai : impossible de le reconsulter tout de suite.
    assert!(game::apply(&c, &mut s, Command::ConsultMentor("jb".into())).is_err());
    // Le mentor principal revient plus vite que les autres.
    let jb = c.mentor("jb").unwrap();
    let mehdi = c.mentor("mehdi").unwrap();
    assert!(dsi_core::mentors::cooldown(&s, jb) < dsi_core::mentors::cooldown(&s, mehdi));
    let lev_before = dsi_core::negotiation::start(&c, &mut s.clone(), "vmware_vsphere", true)
        .unwrap()
        .leverage;
    s.flags.remove("mentor_leverage");
    let lev_after = dsi_core::negotiation::start(&c, &mut s.clone(), "vmware_vsphere", true)
        .unwrap()
        .leverage;
    assert!(lev_before > lev_after);
}

#[test]
fn chaque_mentor_a_toujours_un_conseil() {
    let c = content();
    for m in &c.mentors {
        let s = game::new_game(
            &c,
            21,
            Profile {
                mentor: m.id.clone(),
                ..Profile::default()
            },
        );
        assert!(dsi_core::mentors::advice(&s, m).is_some(), "{}", m.id);
    }
}

#[test]
fn les_anciennes_sauvegardes_se_chargent() {
    let (_, s) = new(22);
    let mut v: serde_json::Value = serde_json::from_str(&save::to_json(&s).unwrap()).unwrap();
    let g = v["game"].as_object_mut().unwrap();
    g.remove("mentors");
    g.remove("tutorial");
    let p = g["profile"].as_object_mut().unwrap();
    p.remove("avatar");
    p.remove("mentor");
    let back = save::from_json(&v.to_string()).unwrap();
    assert_eq!(back.profile.avatar, None);
    assert_eq!(back.tutorial, dsi_core::state::TUTORIAL_OFF);
}

#[test]
fn on_peut_prolonger_la_partie_de_trois_ans() {
    let c = content();
    let mut extended = 0;
    for seed in 0..12 {
        let mut s = game::new_game(&c, seed, Profile::default());
        // Prolonger une partie en cours est refusé.
        assert!(game::apply(&c, &mut s, Command::ExtendCampaign).is_err());
        play_on(&c, &mut s, (seed % 3) as usize);
        let Some(first) = s.ending.clone() else {
            continue;
        };
        assert_eq!(s.cal.year, 3);
        game::apply(&c, &mut s, Command::ExtendCampaign).unwrap();
        assert!(s.ending.is_none() && !s.finished());
        assert_eq!(
            (s.cal.year, s.cal.month),
            (4, 1),
            "la saison 2 commence en janvier de l'année 4"
        );
        assert_eq!(s.past_endings, vec![first]);
        assert_eq!(score::final_year(&c, &s), 6);
        play_on(&c, &mut s, (seed % 3) as usize);
        assert!(
            s.finished(),
            "la partie prolongée {seed} doit se terminer : {:?}",
            s.cal
        );
        if s.ending.is_some() {
            assert_eq!(s.cal.year, 6);
        }
        // Les événements scénarisés de la saison 2 ont été joués.
        assert!(s.seen.contains_key("nis2_compliance") || s.over.is_some());
        extended += 1;
        // Une sauvegarde de partie prolongée se recharge.
        let back = save::from_json(&save::to_json(&s).unwrap()).unwrap();
        assert_eq!(back.extension, 3);
    }
    assert!(extended >= 3, "trop peu de parties prolongées : {extended}");
}

#[test]
fn le_budget_mensuel_se_repartit_entre_salaires_logiciels_et_esn() {
    let (c, mut s) = new(3);
    let b = dsi_core::economy::breakdown(&c, &s);
    assert!(b.salaries > 3.0 && b.software > 4.0 && b.esn > 1.0, "{b:?}");
    assert!((b.balance - dsi_core::economy::monthly_balance(&c, &s)).abs() < 1e-4);
    // Un renfort ESN sur un projet bascule le surcoût dans la colonne ESN.
    s.inbox.clear();
    s.metrics.set(Metric::Budget, 90.0);
    game::apply(&c, &mut s, Command::StartProject("forge".into())).unwrap();
    game::apply(&c, &mut s, Command::Continue).unwrap();
    let before = dsi_core::economy::breakdown(&c, &s);
    let _ = game::apply(&c, &mut s, Command::EsnBoost("forge".into()));
    let after = dsi_core::economy::breakdown(&c, &s);
    assert!(after.esn > before.esn && (after.projects - before.projects).abs() < 1e-4);
    assert!((after.balance - dsi_core::economy::monthly_balance(&c, &s)).abs() < 1e-4);
}

#[test]
fn les_fins_ont_des_objectifs_explicites_et_un_cap() {
    let c = content();
    let (_, mut s) = new(11);
    // Pas de cap avant la saison 2.
    assert!(game::apply(&c, &mut s, Command::SetAmbition(Some("software".into()))).is_err());
    // Sans cap, la fin retenue est la première atteinte dans l'ordre des objectifs.
    for (k, v) in [
        (Metric::Make, 80.0),
        (Metric::Velocity, 60.0),
        (Metric::Sovereignty, 75.0),
        (Metric::VendorDep, 30.0),
    ] {
        s.metrics.set(k, v);
    }
    let goals = score::ending_goals(&s);
    assert!(
        goals
            .iter()
            .all(|g| !g.checks.is_empty() && !g.levers.is_empty())
    );
    let first = goals.iter().find(|g| g.met()).unwrap().id;
    assert_eq!(first, "software");
    assert_eq!(score::ending(&s).id, "software");
    // Le cap l'emporte quand il est atteint, avec un bonus de score.
    s.extension = 3;
    let base = score::ending(&s).score;
    game::apply(&c, &mut s, Command::SetAmbition(Some("sovereign".into()))).unwrap();
    let e = score::ending(&s);
    assert_eq!(e.id, "sovereign");
    assert_eq!(e.ambition, Some(("DSI souveraine".into(), true)));
    assert_eq!(e.score, (base + score::AMBITION_BONUS).min(100));
    // Changer de cap coûte un créneau et du COMEX ; un cap manqué est signalé.
    let (agenda, comex) = (s.agenda, s.m(Metric::Comex));
    game::apply(
        &c,
        &mut s,
        Command::SetAmbition(Some("cloud_native".into())),
    )
    .unwrap();
    assert_eq!(s.agenda, agenda - 1);
    assert!(s.m(Metric::Comex) < comex);
    let e = score::ending(&s);
    assert_eq!(e.id, "software");
    assert_eq!(e.ambition, Some(("Cloud Native Enterprise".into(), false)));
    assert!(game::apply(&c, &mut s, Command::SetAmbition(Some("inconnu".into()))).is_err());
}

#[test]
fn trois_emplacements_de_sauvegarde() {
    assert_eq!(save::SLOT_COUNT, 3);
    let keys: Vec<String> = (0..save::SLOT_COUNT).map(save::slot_key).collect();
    assert_eq!(keys, ["slot1", "slot2", "slot3"]);
    assert!(!keys.iter().any(|k| k == save::AUTOSAVE_KEY));
    let s = play(5, 0);
    let json = save::to_json_at(&s, 1_700_000_000_000).unwrap();
    let (back, at) = save::read(&json).unwrap();
    assert_eq!(at, 1_700_000_000_000);
    assert_eq!(back.cal, s.cal);
    let info = save::slot_info(&back, at);
    assert_eq!(info.date, s.cal.label());
    assert_eq!(info.score, score::score(&s));
    assert!(!info.status.is_empty());
    // Les anciennes sauvegardes (sans horodatage) restent lisibles.
    assert_eq!(save::read(&save::to_json(&s).unwrap()).unwrap().1, 0);
}
