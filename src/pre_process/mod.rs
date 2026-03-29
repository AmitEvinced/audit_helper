use csv::Reader;
use reqwest;
use serde_json::json;
use std::error::Error;
use std::fs::File;
use std::process;


//proccessing the args
#[allow(dead_code)]
pub fn validate_args(args: &Vec<String>) {
    if args.len() < 2 {
        eprintln!("no input path received for the audit");
        process::exit(1);
    }
    
    if args.len() < 3 { 
        eprintln!("no output path recived");
        process::exit(1);
    }
}

//reading the csv 
pub fn read_csv(path: &str) -> Reader<File> {
    let rdr = csv::Reader::from_path(path);
    match rdr {
        Err(e) => {
            eprintln!("Could not read the file. error is : {e}");
            process::exit(1);
        }
        Ok(r) => r,
    }
}

// specifically used for validations.
#[allow(dead_code)]
pub fn create_validation_vectors(
    reader: &mut Reader<File>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut string_vec: Vec<String> = Vec::new();

    for rec in reader.records() {
        let res = rec?;
        let mut temp: String = String::from("");
        temp += "validation: ";
        temp += res.get(0).unwrap();
        temp += "\n";
        temp += "value: ";
        temp += res.get(1).unwrap();
        string_vec.push(temp);
    }
    Ok(string_vec)
}
// creates any from index to content. each item is seperated by a line
pub fn create_general_vector(reader: &mut Reader<File>) -> Vec<String> {
    let mut string_vec = Vec::new();

    for rec in reader.records() {
        let res = rec.expect("could not parse the string");
        let mut temp = String::from("");
        for item in &res {
            temp += item;
            temp += "\n";
        }
        
        string_vec.push(temp);
    }
    string_vec
}

//very long function. calls google to embbedd each validation. and because of limitations. we also
// upload each point one by one. to not use the api key too fast

#[allow(dead_code)]
pub async fn upload_embeddings_to_db(path: &str) -> Result<(), Box<dyn Error>> {
    //reading the csv
    let mut reader = read_csv(path);

    //creating the validations map.
    let string_vec= create_validation_vectors(&mut reader);
    let string_vec = match string_vec {
        Ok(m) => m,
        Err(e) => {
            eprintln!("could not extract the map for some reason, {e}");
            process::exit(1);
        }
    };
    // creating the gemini client via reqwest.
    let gemini_client = reqwest::Client::new();
    let mut counter: u64 = 0;
    for val in string_vec  {
        println!("{val}");
        let body = json!({
            "model": "models/gemini-embedding-001",
            "content": {
                "parts" : [{"text": val}]
            },
            "output_dimensionality": 768 // important for it to match to the db.
        });
        //sending the request to google to get the embbedding for the current validation.
        let _key = std::env::var("GEMINI_API_KEY");
        let req = gemini_client.post("https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent")
        .header("x-goog-api-key", std::env::var("GEMINI_API_KEY").unwrap()) //add error handling later
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

        let res: serde_json::Value = req.json().await?; // this can fail
        let res: &serde_json::Value = &res["embedding"]["values"]; // this can fail if api key does not work can return None

        if let None = res.as_array() {
            print!("Error parsing the the embbedded vector, probably ai studio is down");
            process::exit(1);
        }
        let res = res.as_array().unwrap(); // getting a vector

        // now we need to unpack the vector to f32 (not f64 for speed)
        let res: Vec<f32> = res
            .iter()
            .map(|x| x.as_f64().unwrap())
            .map(|x| x as f32)
            .collect();

        //uploading each vector
        let a = upload_single_to_zilliz(counter, &val, &res).await;
        match a {
            Err(e) => eprintln!("error occurred while uploading {e}"),
            Ok(()) => (),
        }
        counter += 1;
    }

    Ok(())
}



pub async fn upload_single_to_zilliz (index: u64, desc: &String, data: &Vec<f32>) -> Result<(), Box<dyn Error>> {
    
    let body = json!({
        "collectionName": "Validations",
        "data": [
            {
                "primary_key": index,
                "Scaler": desc,
                "vector":  data
            }
        ]
    });
    
   let request = reqwest::Client::new();
   request.post("https://in03-57cec7d9b982bd3.serverless.aws-eu-central-1.cloud.zilliz.com/v2/vectordb/entities/upsert")
       .header("Authorization", "Bearer ".to_string() +&std::env::var("zilliz_api_key").unwrap())
       .header("Content-Type", "application/json")
       .json(&body)
       .send()
       .await?;
    

    Ok(())
}
