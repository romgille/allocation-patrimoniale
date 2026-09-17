//! API HTTP sans état : le questionnaire et les hypothèses arrivent dans la requête,
//! le résultat est calculé à la volée. Aucune donnée n'est conservée.

use std::net::SocketAddr;

use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use engine::{calculer, valider, ErreursValidation, Hypotheses, Questionnaire, Resultat};
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;

#[derive(Debug, Deserialize, Serialize)]
pub struct RequeteCalcul {
    pub questionnaire: Questionnaire,
    pub hypotheses: Option<Hypotheses>,
}

enum Reponse {
    Ok(Box<Resultat>),
    Invalide(ErreursValidation),
}

impl IntoResponse for Reponse {
    fn into_response(self) -> Response {
        match self {
            Reponse::Ok(r) => (StatusCode::OK, Json(r)).into_response(),
            Reponse::Invalide(e) => (StatusCode::UNPROCESSABLE_ENTITY, Json(e)).into_response(),
        }
    }
}

async fn sante() -> &'static str {
    "ok"
}

async fn hypotheses() -> Json<Hypotheses> {
    Json(Hypotheses::default())
}

async fn exemple() -> Json<Questionnaire> {
    Json(Questionnaire::exemple())
}

async fn calcul(corps: Result<Json<RequeteCalcul>, JsonRejection>) -> Reponse {
    let req = match corps {
        Ok(Json(r)) => r,
        Err(e) => {
            return Reponse::Invalide(ErreursValidation {
                erreurs: vec![format!("Requête invalide : {}", e.body_text())],
            })
        }
    };
    let h = req.hypotheses.unwrap_or_default();
    if let Err(erreurs) = valider(&req.questionnaire, &h) {
        return Reponse::Invalide(ErreursValidation { erreurs });
    }
    Reponse::Ok(Box::new(calculer(&req.questionnaire, &h)))
}

pub fn app() -> Router {
    Router::new()
        .route("/api/sante", get(sante))
        .route("/api/hypotheses", get(hypotheses))
        .route("/api/exemple", get(exemple))
        .route("/api/calcul", post(calcul))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(TraceLayer::new_for_http())
}

/// `allocation-api healthcheck` : sonde utilisée par Docker (l'image n'a pas de shell ni de curl).
fn healthcheck() -> ! {
    use std::io::{Read, Write};
    let port = std::env::var("BIND_ADDR")
        .ok()
        .and_then(|a| a.rsplit(':').next().map(str::to_owned))
        .unwrap_or_else(|| "8080".into());
    let ok = (|| -> std::io::Result<bool> {
        let mut s = std::net::TcpStream::connect(format!("127.0.0.1:{port}"))?;
        s.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        s.write_all(b"GET /api/sante HTTP/1.0\r\nHost: localhost\r\n\r\n")?;
        let mut buf = String::new();
        s.read_to_string(&mut buf)?;
        Ok(buf.starts_with("HTTP/1.0 200") || buf.starts_with("HTTP/1.1 200"))
    })()
    .unwrap_or(false);
    std::process::exit(if ok { 0 } else { 1 })
}

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        healthcheck();
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()
        .expect("BIND_ADDR invalide");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    tracing::info!("API d'allocation à l'écoute sur {addr}");
    axum::serve(listener, app())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("serveur");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn post(corps: String) -> (StatusCode, serde_json::Value) {
        let rep = app()
            .oneshot(
                Request::post("/api/calcul")
                    .header("content-type", "application/json")
                    .body(Body::from(corps))
                    .unwrap(),
            )
            .await
            .unwrap();
        let s = rep.status();
        let b = rep.into_body().collect().await.unwrap().to_bytes();
        (s, serde_json::from_slice(&b).unwrap())
    }

    #[tokio::test]
    async fn calcul_ok() {
        let req = RequeteCalcul { questionnaire: Questionnaire::exemple(), hypotheses: None };
        let (s, v) = post(serde_json::to_string(&req).unwrap()).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v["poche_2"]["profil"], "equilibre");
    }

    #[tokio::test]
    async fn calcul_invalide() {
        let mut q = Questionnaire::exemple();
        q.adultes[0].tol_risque = 0;
        let req = RequeteCalcul { questionnaire: q, hypotheses: None };
        let (s, v) = post(serde_json::to_string(&req).unwrap()).await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(v["erreurs"].as_array().unwrap().len() == 1);
    }

    #[tokio::test]
    async fn json_mal_forme() {
        let (s, v) = post("{\"questionnaire\": 1}".into()).await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(v["erreurs"][0].as_str().unwrap().starts_with("Requête invalide"));
    }
}
