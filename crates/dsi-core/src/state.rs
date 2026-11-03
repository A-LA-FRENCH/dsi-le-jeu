//! L'état complet d'une partie : métriques, calendrier, contrats, projets, technos,
//! équipe, file d'événements et scène active. Tout est sérialisable (sauvegarde JSON).

use crate::ai::AiState;
use crate::cloud::CloudState;
use crate::contracts::ContractState;
use crate::negotiation::Negotiation;
use crate::people::{Hire, Interview};
use crate::projects::ProjectState;
use crate::security::WarRoom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Les indicateurs du tableau de bord, tous sur une échelle 0–100.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    Budget,
    Debt,
    Uptime,
    Security,
    Velocity,
    UserSat,
    DevSat,
    Comex,
    VendorDep,
    Sovereignty,
    Make,
    Complexity,
    Attractiveness,
    DataQuality,
    CloudMaturity,
    DevopsMaturity,
    AiMaturity,
    Resilience,
}

impl Metric {
    pub const ALL: [Metric; 18] = [
        Metric::Budget,
        Metric::Security,
        Metric::Debt,
        Metric::Comex,
        Metric::DevSat,
        Metric::Uptime,
        Metric::Velocity,
        Metric::UserSat,
        Metric::VendorDep,
        Metric::Sovereignty,
        Metric::Make,
        Metric::Complexity,
        Metric::Attractiveness,
        Metric::DataQuality,
        Metric::CloudMaturity,
        Metric::DevopsMaturity,
        Metric::AiMaturity,
        Metric::Resilience,
    ];

    /// Les six jauges de la barre du haut.
    pub const PRIMARY: [Metric; 6] = [
        Metric::Budget,
        Metric::Security,
        Metric::Debt,
        Metric::Comex,
        Metric::DevSat,
        Metric::Uptime,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Metric::Budget => "budget",
            Metric::Debt => "debt",
            Metric::Uptime => "uptime",
            Metric::Security => "security",
            Metric::Velocity => "velocity",
            Metric::UserSat => "user_sat",
            Metric::DevSat => "dev_sat",
            Metric::Comex => "comex",
            Metric::VendorDep => "vendor_dep",
            Metric::Sovereignty => "sovereignty",
            Metric::Make => "make",
            Metric::Complexity => "complexity",
            Metric::Attractiveness => "attractiveness",
            Metric::DataQuality => "data_quality",
            Metric::CloudMaturity => "cloud_maturity",
            Metric::DevopsMaturity => "devops_maturity",
            Metric::AiMaturity => "ai_maturity",
            Metric::Resilience => "resilience",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Metric::Budget => "Budget IT",
            Metric::Debt => "Dette technique",
            Metric::Uptime => "Disponibilité",
            Metric::Security => "Sécurité",
            Metric::Velocity => "Vélocité / time-to-market",
            Metric::UserSat => "Satisfaction utilisateurs",
            Metric::DevSat => "Satisfaction développeurs",
            Metric::Comex => "Confiance du COMEX",
            Metric::VendorDep => "Dépendance fournisseurs",
            Metric::Sovereignty => "Souveraineté",
            Metric::Make => "Capacité interne (MAKE)",
            Metric::Complexity => "Complexité",
            Metric::Attractiveness => "Attractivité développeurs",
            Metric::DataQuality => "Qualité des données",
            Metric::CloudMaturity => "Maturité cloud",
            Metric::DevopsMaturity => "Maturité DevOps",
            Metric::AiMaturity => "Maturité IA",
            Metric::Resilience => "Résilience",
        }
    }

    /// Libellé court pour la barre du haut.
    pub fn short(self) -> &'static str {
        match self {
            Metric::Budget => "BUDGET",
            Metric::Debt => "DETTE",
            Metric::Uptime => "UPTIME",
            Metric::Security => "SÉCURITÉ",
            Metric::Velocity => "VÉLOCITÉ",
            Metric::UserSat => "USERS",
            Metric::DevSat => "ENGINEERING",
            Metric::Comex => "COMEX",
            Metric::VendorDep => "LOCK-IN",
            Metric::Sovereignty => "SOUVERAINETÉ",
            Metric::Make => "MAKE",
            Metric::Complexity => "COMPLEXITÉ",
            Metric::Attractiveness => "ATTRACTIVITÉ",
            Metric::DataQuality => "DATA",
            Metric::CloudMaturity => "CLOUD",
            Metric::DevopsMaturity => "DEVOPS",
            Metric::AiMaturity => "IA",
            Metric::Resilience => "RÉSILIENCE",
        }
    }

    /// Vrai quand une valeur élevée est une mauvaise nouvelle.
    pub fn inverted(self) -> bool {
        matches!(self, Metric::Debt | Metric::VendorDep | Metric::Complexity)
    }

    pub fn from_key(k: &str) -> Option<Metric> {
        Metric::ALL.iter().copied().find(|m| m.key() == k)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Metrics(pub BTreeMap<Metric, f32>);

impl Metrics {
    pub fn get(&self, m: Metric) -> f32 {
        self.0.get(&m).copied().unwrap_or(0.0)
    }
    pub fn set(&mut self, m: Metric, v: f32) {
        self.0.insert(m, v.clamp(0.0, 100.0));
    }
    /// Ajoute `d` (borné à 0–100) et renvoie la variation réellement appliquée.
    pub fn add(&mut self, m: Metric, d: f32) -> f32 {
        let before = self.get(m);
        self.set(m, before + d);
        self.get(m) - before
    }
}

/// Les 15 espaces du siège. Chaque événement se déroule dans une salle.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Room {
    Comex,
    #[default]
    Dsi,
    Finance,
    Rh,
    Achats,
    Archi,
    Devs,
    Projet,
    Data,
    Cloud,
    Vendor,
    Crise,
    Cyber,
    Soc,
    Datacenter,
}

impl Room {
    pub const ALL: [Room; 15] = [
        Room::Comex,
        Room::Dsi,
        Room::Finance,
        Room::Rh,
        Room::Achats,
        Room::Archi,
        Room::Devs,
        Room::Projet,
        Room::Data,
        Room::Cloud,
        Room::Vendor,
        Room::Crise,
        Room::Cyber,
        Room::Soc,
        Room::Datacenter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Room::Comex => "Salle du COMEX",
            Room::Dsi => "Bureau DSI",
            Room::Finance => "Direction financière",
            Room::Rh => "Ressources humaines",
            Room::Achats => "Achats",
            Room::Archi => "Architecture",
            Room::Devs => "Open space devs",
            Room::Projet => "Salle projet",
            Room::Data => "Équipe data",
            Room::Cloud => "Cloud control room",
            Room::Vendor => "Vendor meeting room",
            Room::Crise => "Salle de crise",
            Room::Cyber => "Cybersécurité",
            Room::Soc => "SOC",
            Room::Datacenter => "Datacenter",
        }
    }

    /// Position dans la grille du siège (colonne, étage depuis le haut).
    pub fn grid(self) -> (u32, u32) {
        let i = Room::ALL.iter().position(|r| *r == self).unwrap_or(0) as u32;
        (i % 5, i / 5)
    }
}

/// Le temps : 1 tour = 1 mois.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calendar {
    pub year: u32,
    pub month: u32,
}

impl Calendar {
    pub fn start() -> Self {
        Self { year: 1, month: 1 }
    }
    /// Index absolu du mois (année 1 janvier = 0).
    pub fn abs(self) -> u32 {
        (self.year - 1) * 12 + (self.month - 1)
    }
    pub fn from_abs(a: u32) -> Self {
        Self {
            year: a / 12 + 1,
            month: a % 12 + 1,
        }
    }
    pub fn month_name(self) -> &'static str {
        MONTHS[(self.month as usize - 1).min(11)]
    }
    pub fn label(self) -> String {
        format!("{} — Année {}", self.month_name(), self.year)
    }
}

pub const MONTHS: [&str; 12] = [
    "Janvier",
    "Février",
    "Mars",
    "Avril",
    "Mai",
    "Juin",
    "Juillet",
    "Août",
    "Septembre",
    "Octobre",
    "Novembre",
    "Décembre",
];

/// Le profil choisi à la création du ou de la DSI.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub name: String,
    pub female: bool,
    pub company: String,
    pub background: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Camille Durand".into(),
            female: true,
            company: "Groupe Duval-Montaigne".into(),
            background: "ex_dev".into(),
        }
    }
}

/// Ton d'une ligne du fil d'actualité.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    Info,
    Good,
    Bad,
    Money,
    Story,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub turn: u32,
    pub text: String,
    pub tone: Tone,
}

/// Une conséquence programmée : des effets ou un événement, plus tard.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Scheduled {
    pub due: u32,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub effects: crate::effects::Effects,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub origin: String,
}

/// Un élément à traiter ce mois-ci, rattaché à une salle du siège.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Inbox {
    /// Un événement data-driven.
    Event { id: String },
    /// Une conséquence différée qui tombe enfin.
    Consequence {
        title: String,
        text: String,
        effects: crate::effects::Effects,
        room: Room,
    },
    /// Un renouvellement de contrat arrivé à échéance.
    Renewal { contract: String },
    /// Le COMEX de fin d'année.
    YearReview,
    /// Introduction d'un chapitre.
    Chapter { index: usize },
}

/// Résultat affiché après une décision.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Resolution {
    pub title: String,
    pub text: String,
    pub deltas: Vec<(Metric, f32)>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub delayed_hint: bool,
}

/// La scène modale en cours.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Active {
    Event {
        id: String,
    },
    Renewal {
        contract: String,
    },
    Notice {
        title: String,
        text: String,
        effects: crate::effects::Effects,
        room: Room,
    },
    Chapter {
        index: usize,
    },
    Negotiation(Box<Negotiation>),
    Interview(Box<Interview>),
    WarRoom(Box<WarRoom>),
    Review(Box<crate::score::YearReview>),
    Result(Box<Resolution>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameOver {
    Fired,
    CyberApocalypse,
    Bankruptcy,
    LegacyForever,
    VendorLockIn,
    NobodyKnows,
    KubernetesSingularity,
}

impl GameOver {
    pub fn title(self) -> &'static str {
        match self {
            GameOver::Fired => "Vous êtes viré",
            GameOver::CyberApocalypse => "Cyber Apocalypse",
            GameOver::Bankruptcy => "Bankruptcy as a Service",
            GameOver::LegacyForever => "Legacy Forever",
            GameOver::VendorLockIn => "Vendor Lock-In",
            GameOver::NobodyKnows => "Nobody Knows Anything",
            GameOver::KubernetesSingularity => "Kubernetes Singularity",
        }
    }
    pub fn text(self) -> &'static str {
        match self {
            GameOver::Fired => {
                "La DG vous remercie pour votre engagement. Votre badge ne fonctionne plus depuis 8 h 02. Votre successeur est un associé du cabinet de conseil."
            }
            GameOver::CyberApocalypse => {
                "Les écrans affichent une rançon. Le PRA était dans un classeur, dans le datacenter chiffré. L'entreprise fait la une, pas pour les bonnes raisons."
            }
            GameOver::Bankruptcy => {
                "Le budget IT est à zéro. Le DAF a résilié l'abonnement à la machine à café, puis le vôtre."
            }
            GameOver::LegacyForever => {
                "La dette technique a atteint 100 %. Chaque modification casse trois choses. Le SI est désormais classé monument historique."
            }
            GameOver::VendorLockIn => {
                "Vos fournisseurs décident désormais de votre roadmap, de vos prix et de la date de vos congés. Vous êtes un centre de coûts sous licence."
            }
            GameOver::NobodyKnows => {
                "Plus personne en interne ne comprend le SI. Le dernier sachant était un prestataire, parti chez le concurrent."
            }
            GameOver::KubernetesSingularity => {
                "La complexité a atteint un point de non-retour. Le SI compte plus d'abstractions que d'utilisateurs. Même le service mesh a besoin d'un service mesh."
            }
        }
    }
}

/// Une fin de campagne.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Ending {
    pub id: String,
    pub title: String,
    pub text: String,
    pub score: u32,
    pub rank: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GameState {
    pub seed: u64,
    pub rng: ChaCha8Rng,
    pub profile: Profile,
    pub cal: Calendar,
    pub metrics: Metrics,
    /// Allocation budgétaire mensuelle (points de budget par mois).
    pub allocation: f32,
    /// Coût de fonctionnement mensuel de référence au 1er janvier (objectif −15 %).
    pub baseline_run: f32,
    /// Créneaux d'agenda restants ce mois-ci : le temps du ou de la DSI.
    pub agenda: u32,
    pub agenda_max: u32,
    pub techs: BTreeSet<String>,
    pub flags: BTreeSet<String>,
    pub contracts: Vec<ContractState>,
    pub projects: Vec<ProjectState>,
    pub done_projects: BTreeSet<String>,
    pub team: Vec<Hire>,
    pub cloud: CloudState,
    pub ai: AiState,
    /// Événements déjà vus (id → dernier mois absolu).
    pub seen: BTreeMap<String, u32>,
    /// Actions déjà faites (id → dernier mois absolu).
    pub actions_done: BTreeMap<String, u32>,
    pub decisions: Vec<String>,
    pub scheduled: Vec<Scheduled>,
    pub inbox: Vec<Inbox>,
    pub active: Option<Active>,
    pub log: Vec<LogEntry>,
    pub over: Option<GameOver>,
    pub ending: Option<Ending>,
    pub year_scores: Vec<u32>,
    pub debug: bool,
}

impl GameState {
    pub fn new(seed: u64, profile: Profile) -> Self {
        let mut metrics = Metrics::default();
        for (m, v) in [
            (Metric::Budget, 50.0),
            (Metric::Debt, 62.0),
            (Metric::Uptime, 74.0),
            (Metric::Security, 46.0),
            (Metric::Velocity, 28.0),
            (Metric::UserSat, 45.0),
            (Metric::DevSat, 35.0),
            (Metric::Comex, 55.0),
            (Metric::VendorDep, 72.0),
            (Metric::Sovereignty, 22.0),
            (Metric::Make, 18.0),
            (Metric::Complexity, 58.0),
            (Metric::Attractiveness, 25.0),
            (Metric::DataQuality, 30.0),
            (Metric::CloudMaturity, 8.0),
            (Metric::DevopsMaturity, 10.0),
            (Metric::AiMaturity, 3.0),
            (Metric::Resilience, 32.0),
        ] {
            metrics.set(m, v);
        }
        Self {
            seed,
            rng: ChaCha8Rng::seed_from_u64(seed),
            profile,
            cal: Calendar::start(),
            metrics,
            allocation: 8.8,
            baseline_run: 0.0,
            agenda: 3,
            agenda_max: 3,
            techs: BTreeSet::new(),
            flags: BTreeSet::new(),
            contracts: Vec::new(),
            projects: Vec::new(),
            done_projects: BTreeSet::new(),
            team: Vec::new(),
            cloud: CloudState::default(),
            ai: AiState::default(),
            seen: BTreeMap::new(),
            actions_done: BTreeMap::new(),
            decisions: Vec::new(),
            scheduled: Vec::new(),
            inbox: Vec::new(),
            active: None,
            log: Vec::new(),
            over: None,
            ending: None,
            year_scores: Vec::new(),
            debug: false,
        }
    }

    pub fn m(&self, m: Metric) -> f32 {
        self.metrics.get(m)
    }

    pub fn turn(&self) -> u32 {
        self.cal.abs()
    }

    pub fn has_tech(&self, t: &str) -> bool {
        self.techs.contains(t)
    }

    pub fn has_flag(&self, f: &str) -> bool {
        self.flags.contains(f)
    }

    pub fn push_log(&mut self, text: impl Into<String>, tone: Tone) {
        let turn = self.turn();
        self.log.push(LogEntry {
            turn,
            text: text.into(),
            tone,
        });
        if self.log.len() > 300 {
            let excess = self.log.len() - 300;
            self.log.drain(..excess);
        }
    }

    /// Personnalise un texte : {dsi}, {company}, {le_la}, {e} (accord).
    pub fn tx(&self, s: &str) -> String {
        let p = &self.profile;
        s.replace("{dsi}", &p.name)
            .replace("{company}", &p.company)
            .replace("{le_la}", if p.female { "la" } else { "le" })
            .replace("{Le_La}", if p.female { "La" } else { "Le" })
            .replace("{e}", if p.female { "e" } else { "" })
    }

    /// Est-ce que la partie attend une décision (rien d'autre ne peut avancer) ?
    pub fn busy(&self) -> bool {
        self.active.is_some()
    }

    pub fn finished(&self) -> bool {
        self.over.is_some() || self.ending.is_some()
    }

    /// Vérifie les conditions de défaite après chaque application d'effets.
    pub fn check_game_over(&mut self) {
        if self.over.is_some() || self.ending.is_some() {
            return;
        }
        let m = &self.metrics;
        let over = if m.get(Metric::Comex) <= 0.0 {
            Some(GameOver::Fired)
        } else if m.get(Metric::Security) <= 0.0 {
            Some(GameOver::CyberApocalypse)
        } else if m.get(Metric::Budget) <= 0.0 {
            Some(GameOver::Bankruptcy)
        } else if m.get(Metric::Debt) >= 100.0 {
            Some(GameOver::LegacyForever)
        } else if m.get(Metric::VendorDep) >= 100.0 {
            Some(GameOver::VendorLockIn)
        } else if m.get(Metric::Make) <= 0.0 {
            Some(GameOver::NobodyKnows)
        } else if m.get(Metric::Complexity) >= 100.0 {
            Some(GameOver::KubernetesSingularity)
        } else {
            None
        };
        if let Some(o) = over {
            self.over = Some(o);
            self.push_log(format!("GAME OVER — {}", o.title()), Tone::Bad);
        }
    }
}
