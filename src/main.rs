mod actual_work;
mod pre_process;
mod clients_connection;

use std::process;
//use std::env;
use tokio;
use std::io;

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

    //getting args
    let mut path = String::new();
    println!("please enter the file path");
    io::stdin().read_line(&mut path).expect("did not enter a correct path");
    let path = path.trim().to_string();
    
    /* let args: Vec<String> = env::args().collect();
    pre_process::validate_args(&args);

    let path = &args[1];
    */

    //reading the file
    let mut reader = pre_process::read_csv(&path);

    //getting the map
    let issues_vector = pre_process::create_general_vector(&mut reader);

    //pre_process::create_qdrant_db().await; //this part should only be used once because it's the creating of the data base.

    //pre_process::create_vector_map("../validations.csv").await; // upload the vectos to db
    
    //creating the sheet with all of the responses
    actual_work::create_response_sheet(issues_vector).await;
}
