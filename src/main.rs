mod actual_work;
mod pre_process;

use std::process;
use std::env;
use tokio;

#[tokio::main]
async fn main() {
    // must add this line otherwise crypto provider is not installed and can not connect to qdrant.
    let crypto_provider = rustls::crypto::ring::default_provider().install_default();
    match crypto_provider {
        Err(e) => {
            println!("failed to establish crypto provider {e:?}");
            process::exit(1);
        }
        _ => (),
    }

    let args: Vec<String> = env::args().collect();
    pre_process::validate_args(&args);

    let path = &args[1];

    //reading the file
    let mut reader = pre_process::read_csv(path);

    //getting the map
    let id_to_issue_map = pre_process::create_general_map(&mut reader);

    //pre_process::create_qdrant_db().await; //this part should only be used once because it's the creating of the data base.

    //pre_process::create_vector_map("../validations.csv").await; // upload the vectos to db

    let result = actual_work::get_closest(id_to_issue_map.get(&0).unwrap()).await;

    let response = actual_work::qeury_to_gemini(id_to_issue_map.get(&0).unwrap(), result).await;
    println!("{}", response);
}
