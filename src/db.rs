use mongodb::{Client, options::ClientOptions};
use tokio::task;

pub async fn db_control() {
	let url = "mongodb+srv://gleb:KUKJv7Xzs2gjQnU2@datastorage-mu2ho.mongodb.net/test?retryWrites=true&w=majority";
	let options = ClientOptions::parse(url).await.expect("failed to parse options");
	let client = Client::with_options(options).expect("failed to set client");
	let client_clone = client.clone();

	task::spawn(async move{
		db_control_process(client_clone).await;
	});

	task::yield_now().await;
}

async fn db_control_process(client: Client) {
	// List the names of the databases in that deployment.
	for db_name in client.list_database_names(None, None).await.unwrap() {
    	println!("{}", db_name);
	}
}