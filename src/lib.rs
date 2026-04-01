mod pre_process;
mod actual_work;
mod clients_connection;

use axum::response::IntoResponse;
use serde_json::json;
use axum::Json;
use reqwest::StatusCode;

pub async fn run_audit(input_path: &str, output_path: &str) -> Result<(), ApiError>{
        //reading the file
    let reader = pre_process::read_csv(input_path);
    let mut reader = match reader {
        Ok(r) => r,
        Err(e) => return Err(ApiError::InternalError(e.to_string()))
    };

   //getting the map
    let issues_vector = pre_process::create_general_vector(&mut reader);
    let issues_vector = match issues_vector {
        Ok(vec) => vec, 
        Err(e) =>  return Err(e),
    };


    //pre_process::upload_embeddings_to_db("../validations.csv").await; // upload the vectos to db

    //creating the sheet with all of the responses
    let  work = actual_work::create_response_sheet(issues_vector, output_path).await;
    match work {
        Ok(()) => (),
        Err(e) => return Err(e),
    }


    
    Ok(())
}
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    InvalidInput(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": error_message,

        }));

        (status, body).into_response()
    }
}