use reqwest;
use serde_json::Value;
use serde_json::json;
use std::error::Error;

pub async fn connect_to_gemini_client(body: &Value, embedd_url: &str) -> Result<reqwest::Response, Box<dyn Error + Send + Sync>> {
    let request = reqwest::Client::new(); //building client for reqwest
    let req = request.post(embedd_url)
   .header("x-goog-api-key", std::env::var("GEMINI_API_KEY").unwrap())
   .header("Content-Type", "application/json")
   .json(body)
   .send()
   .await?;

    Ok(req)
}

pub fn create_body(query: &String, context_string: &String) -> Value {

    //creating input for gemini
    let role_string = "you are acting as an accessibility engineer 
                    (Evinced-style). Given (A) an issue description and (B) a list of candidate validation 
                rules from retrieval, decide which rule(s), if any, plausibly detect or cover this issue,
                 and briefly why";

    let description_string = "### Issue description \n".to_string() + query;

    let context = "Here is a list of the possible validations: \n".to_string() + context_string;

    let body = json!({ //building the body of the request to gemini
        "model": "models/gemini-3-flash-preview",
        "contents" : [
            {
                "role": "user",
                "parts" :[{"text": role_string }]
            },

            {
                "role": "user",
                "parts" : [{"text": description_string}]
            },

            {
                "role" : "user",
                "parts" : [{"text" : context}]

            },

            {
                "role" : "user",
                "parts" : [{"text" : "Return format:
                if you thing a validation fits, return the name of the validation with a short explanation of why
                if you don't think any of them fit, return the None
                if you think I should decide, return the name of the validation and ###Menual check###"}]
            }
        ]
        }
    );
    body
}
