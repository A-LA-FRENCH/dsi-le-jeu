# Contribuer à DSI — Le Jeu

Merci ! Le plus simple pour contribuer : **du contenu**. Les événements, fournisseurs,
technologies, projets, questions d'entretien et crises vivent dans `data/*.json` ; aucun
changement de moteur n'est nécessaire (voir le README, section « Ajouter du contenu »).

## Règles d'écriture

- Dialogues courts : 1 à 4 phrases. Drôles parce que crédibles, pas l'inverse.
- **Chaque décision doit faire mal quelque part.** Pas d'option gratuite.
- Aucune technologie magique : bénéfices, coûts, dépendances, compétences.
- Sociétés réelles : uniquement comme repères du marché, en texte, sans logo. Ne jamais
  présenter comme un fait une pratique commerciale négative inventée ; les répliques des
  commerciaux restent génériques et clairement fictives.
- Cybersécurité : prévention, arbitrage, réponse. Jamais de méthode offensive exploitable.

## Avant une pull request

```bash
cargo test                                   # valide aussi toutes les références des données
cargo clippy -- -D warnings
cargo run -p dsi-core --example balance --release   # si vous touchez à l'équilibrage
RENDERERS=webgl2 PROFILE=dev tools/build-web.sh     # si vous touchez au client
```

Le moteur de simulation (`crates/dsi-core`) doit rester indépendant du rendu et déterministe
(même graine, même partie) : pas d'horloge, pas d'aléa hors `GameState::rng`.
