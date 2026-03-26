use csv::Writer;
use qdrant_client::qdrant::value::Kind;
use qdrant_client::{
    self, Qdrant,
    qdrant::{Query, QueryPointsBuilder},
};
use reqwest;
use serde_json::json;
use std::fs::File;
use std::process;
use std::rc::Rc;

pub async fn get_closest(query: &str) -> Vec<String> {
    // get the embedding
    let embbedded_vec = vectorize_query(query).await;

    //establish qdrant conntection which can fail
    let client = Qdrant::from_url(std::env::var("URL").unwrap().as_str())
        .api_key(std::env::var("API_KEY"))
        .build();

    let client = match client {
        Err(e) => {
            println!("Could not connect to qdrant {e}");
            process::exit(1);
        }
        Ok(x) => x,
    };

    //qeury on the results
    let result = client
        .query(
            QueryPointsBuilder::new("validation_data_set")
                .query(Query::new_nearest(embbedded_vec))
                .limit(3)
                .with_payload(true),
        )
        .await;


    let result = match result {
        Err(e) => {
            println!("Error while searching for points {e}");
            process::exit(1);
        }
        Ok(x) => x,
    };

    let mut vector_result: Vec<String> = Vec::new();

    //extracting the resulst from qdrant into a vector of Strings for ease of use.
    let x = result.result.iter();
    for point in x {
        let payload = &point.payload;
        let data = payload.get("data").unwrap();
        let kind = data.kind.as_ref();
        let data = match kind {
            Some(Kind::StringValue(s)) => s,
            _ => {
                println!("Couldnot extract the string");
                process::exit(1);
            }
        };
    
        vector_result.push(data.to_string());
    }
    // returning all of the validations found in qdrant db
    vector_result
}

pub async fn qeury_to_gemini(query: &String, context: Vec<String>) -> Rc<String> {
    let request = reqwest::Client::new(); //building client for reqwest

    let mut context_string: String = String::from("");
    for val in context { //creating the string with out validations
        context_string += &val;
        context_string += "\n";
    }

    //creating input for gemini
    let role_string = "you are acting as an accessibility engineer 
                    (Evinced-style). Given (A) an issue description and (B) a list of candidate validation 
                rules from retrieval, decide which rule(s), if any, plausibly detect or cover this issue,
                 and briefly why";

    let description_string = "### Issue description \n".to_string() + &query;

    let context = "Here is a list of the possible validations: \n".to_string() + &context_string;

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
                "parts" : [{"text" : "Return format: a simple yes + validation name
                if you decide that one of our validatons in a fit or None if they don't.
                if you think I need to decide menually, please mention that"}]
            }
        ]
        }
    );

    //calling gemini
    let req = request.post("https://generativelanguage.googleapis.com/v1beta/models/gemini-3-flash-preview:generateContent")
   .header("x-goog-api-key", std::env::var("GEMINI_API_KEY").unwrap())
   .header("Content-Type", "application/json")
   .json(&body)
   .send()
   .await;

    let res = match req {
        Err(e) => {
            println!("could not send request to google {e}");
            process::exit(1);
        }
        Ok(x) => x,
    };

    let res: Result<serde_json::Value, reqwest::Error> = res.json().await; // getting the json out of the response

    let res: serde_json::Value = match res {
        Err(e) => {
            println!("Error extracting the json out of the gemini request {e}");
            process::exit(1);
        }
        Ok(x) => x,
    };

    if res.get("error").is_some() {
        println!("we got an error code in the gemini request, try again");
        process::exit(1);
    }

    //extracting the values out of the json response
    let res: Option<&serde_json::Value> = res["candidates"]
        .get(0)
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|part| part.get("text"));

    let res = match res {
        Some(t) => t,
        None => {
            println!("error while parsing the gemini response json. It should work so try again");
            process::exit(1);
        }
    };

    // must use Rc if we want to return a String
    let res: Rc<String> = match res {
        serde_json::Value::String(a) => Rc::new(a.to_string()),
        _ => process::exit(1),
    };
    res
}

async fn vectorize_query(query: &str) -> Vec<f32> {
    let gemini_client = reqwest::Client::new(); //creating gemini client 

    let body = json!({
        "model": "models/gemini-embedding-001",
        "content": {
            "parts" : [{"text": query}]
        },
        "output_dimensionality": 768 // important for it to match to qdrant.
    });

    //sending the request to google to get the embbedding for the current validation.
    let req = gemini_client.post("https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent")
        .header("x-goog-api-key", std::env::var("GEMINI_API_KEY").unwrap()) //add error handling later
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;

    let res = match req {
        Err(e) => {
            println!("could not send request to google {e}");
            process::exit(1);
        }
        Ok(x) => x,
    };


    let res: serde_json::Value = res.json().await.expect("Could not get json"); // this can fail

    let res: &serde_json::Value = &res["embedding"]["values"]; // this can fail if api key does not work can return None

    if let None = res.as_array() {
        print!("Error parsing the the embbedded vector, probably ai studio is down");
        process::exit(1);
    }

    //extracting the vector out
    let res = res.as_array().unwrap();
    let res: Vec<f32> = res
        .iter()
        .map(|x| x.as_f64().unwrap())
        .map(|x| x as f32)
        .collect();

    res
}

// creating the cvs with the resutls
pub async fn create_response_sheet(vector: Vec<String>) {
    let mut counter = 0;
    let wrt = Writer::from_path("../results.csv");
    let mut wrt: Writer<File> = match wrt {
        Err(e) => {
            println!("Error creating csv  {e}");
            process::exit(1);
        }
        Ok(x) => x,
    };
    //header row
    let head_res = wrt.write_record(&["response"]);
    match head_res {
        Err(e) => {
            println!("failed to create header row {e}");
            process::exit(1);
        }
        Ok(()) => (),
    }
    //going over all of the issues
    for issue in vector {
        counter += 1;
        let validations = get_closest(&issue).await;
        let respose = qeury_to_gemini(&issue, validations).await;

        let write_res = wrt.write_record(&[respose.as_str()]);
        match write_res {
            Err(e) => {
                println!("Error creating row {e}");
                continue;
            }
            Ok(()) => {
                println!("created {counter}")
            }
        }
        // I want to flush after each load, to not lose data in case of a failirue
        let flush = wrt.flush();
        match flush {
            Err(e) => {
                println!("could not flush writer for some reason {e}");
                continue;
            }
            Ok(()) => (),
        }
        // just for testing to not overload api key
        if counter == 3 {
            break;
        }
    }
}
