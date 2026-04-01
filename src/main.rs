use audit_agent::run_audit;
use axum::Json;
use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::json;
//use std::env;
use audit_agent::ApiError;

#[warn(clippy::pedantic)]

#[tokio::main]
async fn main() {
    //let args: Vec<String> = env::args().collect();
    //pre_process::validate_args(&args);

    //let input_path = &args[1];
    //let output_path = &args[2];
    //run_audit(input_path, output_path).await;

    let app = create_app();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind tcp listener");

    println!("Server running on http://localhost:3000");

    axum::serve(listener, app)
        .await
        .expect("failed to start server");
}

fn create_app() -> Router {
    Router::new()
        .route("/audit", post(get_audit))
        .route("/", get(health))
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status" : "ok" }))
}

async fn get_audit(
    payload: Result<Json<AuditPath>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let payload = match payload {
        Ok(payload) => payload,
        Err(rej) => {
            return Err(ApiError::InvalidInput(rej.to_string()));
        }
    };
    
    let Json(payload) = payload;
    let input_path = &payload.input_path;
    let output_path = &payload.output_path;

    let res = run_audit(input_path, output_path).await;
    match res {
        Ok(()) => Ok(Json(json!({
            "status": "success",
            "message" :"created the file"
        }))),
        
        Err(e) => Err(e),
        
    }
}

#[derive(Deserialize)]
struct AuditPath {
    input_path: String,
    output_path: String,
}
