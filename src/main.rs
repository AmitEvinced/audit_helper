mod actual_work;
mod clients_connection;
mod pre_process;

use std::env;
use tokio;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = env::args().collect();
    pre_process::validate_args(&args);

    let input_path = &args[1];
    let output_path = &args[2];
    run_audit(input_path, output_path).await;
}

pub async fn run_audit(input_path: &String, output_path: &String) {
        //reading the file
    let mut reader = pre_process::read_csv(input_path);

    //getting the map
    let issues_vector = pre_process::create_general_vector(&mut reader);


    //pre_process::upload_embeddings_to_db("../validations.csv").await; // upload the vectos to db

    //creating the sheet with all of the responses
    actual_work::create_response_sheet(issues_vector, output_path).await;
}
