//! Le moteur exposé au navigateur, sans serveur : même contrat que l'API HTTP
//! (`/api/hypotheses`, `/api/exemple`, `/api/calcul`), mais en appels de fonctions.
//! Les échanges se font en JSON (chaînes) pour rester identiques à l'API.

use engine::{calculer as calculer_moteur, valider, Hypotheses, Questionnaire, Resultat};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
struct RequeteCalcul {
    questionnaire: Questionnaire,
    hypotheses: Option<Hypotheses>,
}

/// Même forme que `ReponseCalcul` côté TypeScript.
#[derive(Serialize)]
#[serde(untagged)]
enum Reponse {
    Ok { ok: bool, resultat: Box<Resultat> },
    Invalide { ok: bool, erreurs: Vec<String> },
}

fn json<T: Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("sérialisation JSON")
}

/// Hypothèses par défaut (équivalent de `GET /api/hypotheses`).
#[wasm_bindgen]
pub fn hypotheses() -> String {
    json(&Hypotheses::default())
}

/// Questionnaire d'exemple (équivalent de `GET /api/exemple`).
#[wasm_bindgen]
pub fn exemple() -> String {
    json(&Questionnaire::exemple())
}

/// Calcul complet (équivalent de `POST /api/calcul`).
/// Entrée : `{ "questionnaire": …, "hypotheses": … | null }`.
#[wasm_bindgen]
pub fn calculer(requete: &str) -> String {
    let req: RequeteCalcul = match serde_json::from_str(requete) {
        Ok(r) => r,
        Err(e) => {
            return json(&Reponse::Invalide {
                ok: false,
                erreurs: vec![format!("Requête invalide : {e}")],
            })
        }
    };
    let h = req.hypotheses.unwrap_or_default();
    if let Err(erreurs) = valider(&req.questionnaire, &h) {
        return json(&Reponse::Invalide { ok: false, erreurs });
    }
    json(&Reponse::Ok { ok: true, resultat: Box::new(calculer_moteur(&req.questionnaire, &h)) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json as j, Value};

    fn appel(v: Value) -> Value {
        serde_json::from_str(&calculer(&v.to_string())).unwrap()
    }

    #[test]
    fn calcul_ok() {
        let q: Value = serde_json::from_str(&exemple()).unwrap();
        let v = appel(j!({ "questionnaire": q }));
        assert_eq!(v["ok"], true);
        assert_eq!(v["resultat"]["poche_2"]["profil"], "equilibre");
    }

    #[test]
    fn hypotheses_explicites() {
        let q: Value = serde_json::from_str(&exemple()).unwrap();
        let h: Value = serde_json::from_str(&hypotheses()).unwrap();
        assert_eq!(appel(j!({ "questionnaire": q, "hypotheses": h }))["ok"], true);
    }

    #[test]
    fn calcul_invalide() {
        let mut q: Value = serde_json::from_str(&exemple()).unwrap();
        q["adultes"][0]["tol_risque"] = j!(0);
        let v = appel(j!({ "questionnaire": q }));
        assert_eq!(v["ok"], false);
        assert_eq!(v["erreurs"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn json_mal_forme() {
        let v = appel(j!({ "questionnaire": 1 }));
        assert_eq!(v["ok"], false);
        assert!(v["erreurs"][0].as_str().unwrap().starts_with("Requête invalide"));
    }
}
