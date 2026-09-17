# Allocation patrimoniale

Outil web local qui applique la logique d'allocation « études des enfants + revenus passifs »
(résident fiscal français, paramètres septembre 2026) à partir d'un questionnaire.

> Outil informatif et pédagogique : ce n'est pas un conseil en investissement.

## Démarrer

Il faut Docker et Docker Compose.

```bash
docker compose up -d --build
# puis ouvrir http://localhost:8080
```

- Autre port : `WEB_PORT=9000 docker compose up -d --build`
- Arrêter : `docker compose down`

Le port n'est publié que sur `127.0.0.1` (usage local, sans authentification).

## Architecture

```
navigateur ──► web (nginx, non root) ──/api──► api (Rust/axum, distroless, non root)
               fichiers statiques React        moteur de calcul sans état
```

| Dossier | Contenu |
|---|---|
| `crates/engine` | Moteur de calcul en Rust : types d'entrée/sortie, poches 0/1/2, profils, modificateurs, garde-fous, répartition des flux, alertes. Testé unitairement. |
| `crates/api` | API HTTP axum : `GET /api/hypotheses`, `GET /api/exemple`, `POST /api/calcul`, `GET /api/sante`. |
| `web` | Front React + TypeScript (strict). Les types de `web/src/bindings` sont **générés depuis Rust** avec ts-rs : le contrat front/back est vérifié à la compilation. |
| `docker` | Dockerfiles multi-étapes et configuration nginx (CSP, en-têtes de sécurité). |

**Sans état** : le serveur ne stocke rien. Les exports de la première version (un seul profil
pour le foyer) sont convertis automatiquement à l'import. Le navigateur garde un brouillon local
(`localStorage`), et les boutons **Exporter / Importer** produisent et relisent un fichier JSON
(questionnaire + hypothèses).

## Utilisation

1. **Questionnaire** (6 étapes), rempli pour tout le foyer, avec un aperçu qui se met à jour en direct :
   - **Foyer** : un bloc par adulte (âge, stabilité des revenus, tolérance au risque, temps
     disponible, abondement employeur), plus la TMI, la résidence principale et le LEP ;
   - **Budget** : revenus de chaque adulte, dépenses par poste (logement, alimentation,
     transport, enfants…), crédits. La capacité d'épargne se calcule seule ; on peut imposer un
     autre montant ;
   - **Enfants & projets** : études par enfant et projets datés (voiture, travaux, voyage,
     apport…) avec montant, échéance, priorité et somme déjà mise de côté ;
   - **Revenus passifs**, **Patrimoine**, **Préférences**.
2. **Résultats** : budget du foyer (où va le revenu, part de chaque adulte, dépenses par poste),
   alertes classées par gravité, répartition de l'épargne mensuelle, poche 0,
   sous-poches datées (études et projets) avec leur glide path, le besoin et le montant
   réellement versé, allocation cible et actuelle de la poche 2 avec
   apports par ligne, enveloppes et supports, trajectoire du capital, leviers si l'objectif
   n'est pas atteignable, phase revenus passifs.
3. **Hypothèses** : tous les paramètres « 2026 » sont modifiables (frais d'études, rendements,
   glide path, profils, seuils, plafonds, fiscalité). Un bouton rétablit les valeurs par défaut.

## Développement

```bash
make test        # tests Rust + régénération des types TypeScript
make dev-api     # API sur :8080
make dev-web     # front Vite sur :5173 (relaie /api vers :8080)
make check       # tests, clippy et typecheck du front
```

Après toute modification d'un type Rust exposé, lancer `make types` (ou `cargo test -p engine`) :
les fichiers de `web/src/bindings` sont régénérés et le compilateur TypeScript signale ce qu'il
faut adapter.

La CI GitHub (`.github/workflows/ci.yml`) lance les tests, clippy et le build du front. Elle
vérifie que les types générés sont à jour, construit les images Docker et fait un test de fumée
via `docker compose up`.

## Saisie en famille : règles retenues

- **Profil de risque** : la tolérance la plus faible des adultes est retenue. Une alerte apparaît
  si les réponses diffèrent de 2 points ou plus. La stabilité des revenus est une moyenne
  pondérée par les revenus de chacun. L'âge de référence est celui de l'adulte le plus âgé
  (règle « 110 − âge »). Le temps de gestion retenu est le plus faible. L'abondement employeur
  compte dès qu'un adulte y a droit.
- **Capacité d'épargne** = revenus − dépenses courantes − mensualités de crédit, sauf si un
  montant est imposé. La précaution se calcule sur dépenses + mensualités.
- **Projets datés** : même glide path et même formule PMT que les études. Leur montant est
  indexé sur une inflation de 2 % (modifiable). Si l'épargne ne suffit pas, les études
  (toujours « essentiel ») et les projets essentiels sont servis d'abord, puis les importants,
  puis les souhaitables, au prorata à l'intérieur d'un même niveau.
- **Objectif revenus passifs** : il est jugé atteignable par simulation mois par mois.
  L'épargne comble d'abord la précaution et les crédits chers, puis finance les projets en cours.
  Chaque projet arrivé à échéance libère son versement pour la poche 2.

## Choix d'interprétation de la spécification

Là où la spécification laisse une marge, voici ce que fait le moteur (tout est modifiable dans
**Hypothèses** ou dans `crates/engine`) :

- **Précaution** : 6 mois si les revenus sont instables, 3 mois si la RP est payée et les
  revenus stables, **4 mois** sinon.
  Un excédent de livrets est redéployé dans la poche 2.
- **Dettes > 5 %** : le reste du flux mensuel y est affecté (dans la limite du restant dû),
  avant les études et projets, comme dans l'ordre de la section 5.
- **Frais d'études par défaut** : 3 000 €/an public, 10 000 €/an privé, 6 500 €/an mixte,
  9 000 €/an de logement. Le glide path s'applique en années entières : > 12 ans devient ≥ 13 ans.
  Un enfant de 18 ans ou plus a un horizon nul : aucun versement mensuel n'est calculé et le
  manque éventuel est signalé.
- **Base de la poche 2** = placements financiers (hors livrets et hors capital affecté aux études et projets)
  + locatif net. L'immobilier regroupe le locatif net et les SCPI, le crowdfunding regroupe
  l'immobilier et l'ENR. Le plafond immobilier total inclut en plus le crowdfunding immobilier.
- **Capital pris en compte pour la formule PMT** : tout le financier de la poche 2
  (hors locatif direct, dont les revenus sont déjà dans le cash-flow) + les liquidités à investir.
- **Rendement de la poche 2** : celui du profil retenu (4 / 5,5 / 6,5 %).
- **Garde-fous** : les résidus de renormalisation vont d'abord vers l'ETF monde (dans la limite
  du plafond actions), puis vers les fonds euros. Au-dessus du plafond actions, l'excédent passe
  en fonds euros.
- **Lignes fermées aux apports** : cible à 0 (crypto, private equity par défaut), immobilier figé
  (locatif > 40 % du patrimoine) ou au-dessus du plafond, crowdfunding au-dessus de 5 %.
- **Répartition des apports** : on réduit d'abord les écarts les plus négatifs (en euros), en les
  nivelant. Une fois les cibles atteintes, le surplus est réparti au prorata des cibles.
- **Crypto** : 0 par défaut. Si vous en voulez, elle est plafonnée à 3 % et prise sur la ligne
  actions en direct.
- **Private equity** : compté dans les satellites, sans nouvel apport.
