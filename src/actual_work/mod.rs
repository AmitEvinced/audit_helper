use csv::Writer;
use reqwest;
use serde_json::json;
use std::error::Error;
use std::fs::File;
use std::process;
use std::rc::Rc;

pub async fn get_closest_zilliz(query: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let embbedded_vec = vectorize_query(query).await;

    let body = json!({
        "collectionName": "Validations",
        "data": [embbedded_vec],
        "annsField": "vector",
        "limit": 3,
        "outputFields": [
            "*"
        ]
    });

    let request = reqwest::Client::new();
    let res =request.post("https://in03-57cec7d9b982bd3.serverless.aws-eu-central-1.cloud.zilliz.com/v2/vectordb/entities/search")
       .header("Authorization", "Bearer ".to_string() +&std::env::var("zilliz_api_key").unwrap())
       .header("Content-Type", "application/json")
       .json(&body)
       .send()
       .await?;

    let mut result_vec: Vec<String> = Vec::new();
    let result: serde_json::Value = res.json().await?;

    if let Some(rows) = result["data"].as_array() {
        for row in rows {
            if let Some(text) = row["Scaler"].as_str() {
                result_vec.push(text.to_string());
            }
        }
    }

    Ok(result_vec)
}

pub async fn qeury_to_gemini(query: &String, context: Vec<String>) -> Rc<String> {
    let mut context_string: String = String::from("");
    for val in context {
        //creating the string with out validations
        context_string += &val;
        context_string += "\n";
    }

    let body = crate::clients_connection::create_body(query, &context_string);

    //calling gemini
    let req = crate::clients_connection::connect_to_gemini_client(&body,"https://generativelanguage.googleapis.com/v1beta/models/gemini-3-flash-preview:generateContent").await;
    let res = match req {
        Err(e) => {
            eprintln!("could not send request to google {e}");
            process::exit(1);
        }
        Ok(x) => x,
    };

    let res: Result<serde_json::Value, reqwest::Error> = res.json().await; // getting the json out of the response

    let res: serde_json::Value = match res {
        Err(e) => {
            eprintln!("Error extracting the json out of the gemini request {e}");
            process::exit(1);
        }
        Ok(x) => x,
    };

    if res.get("error").is_some() {
        eprintln!("we got an error code in the gemini request, try again");
        println!("{:?}", res["error"]["message"].as_str());
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
            eprintln!("error while parsing the gemini response json. It should work so try again");
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
    let body = json!({
        "model": "models/gemini-embedding-001",
        "content": {
            "parts" : [{"text": query}]
        },
        "output_dimensionality": 768 // important for it to match to db.
    });

    //sending the request to google to get the embbedding for the current validation.
    let req = crate::clients_connection::connect_to_gemini_client(
        &body,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent",
    )
    .await;

    let res = match req {
        Err(e) => {
            eprintln!("could not send request to google {e}");
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
pub async fn create_response_sheet(vector: Vec<String>, out_put_path: &String) {
    let mut counter = 0;
    let wrt = Writer::from_path(out_put_path);
    let mut wrt: Writer<File> = match wrt {
        Err(e) => {
            eprintln!("Error creating csv  {e}");
            process::exit(1);
        }
        Ok(x) => x,
    };
    //header row
    let head_res = wrt.write_record(&["response"]);
    match head_res {
        Err(e) => {
            eprintln!("failed to create header row {e}");
            process::exit(1);
        }
        Ok(()) => (),
    }
    //going over all of the issues
    for issue in vector {
        counter += 1;
        let validations = get_closest_zilliz(&issue).await;
        let validations = match validations {
            Err(e) => {
                println!("Error finding closests, {e}");
                process::exit(1);
            }
            Ok(x) => x,
        };
        let respose = qeury_to_gemini(&issue, validations).await;

        let write_res = wrt.write_record(&[respose.as_str()]);
        match write_res {
            Err(e) => {
                eprintln!("Error creating row {e}");
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
                eprintln!("could not flush writer for some reason {e}");
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
