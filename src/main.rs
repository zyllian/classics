#![feature(lazy_cell)]

use std::path::PathBuf;

use error::GeneralError;
use server::{
	config::{OptionalServerConfig, ServerConfig},
	Server,
};

mod command;
mod error;
mod level;
mod packet;
mod player;
mod server;
mod util;

const SERVER_NAME: &str = "classics";
const CONFIG_FILE: &str = "./server-config.json";

#[tokio::main]
async fn main() -> Result<(), GeneralError> {
	let config_path = PathBuf::from(CONFIG_FILE);
	let config = if config_path.exists() {
		serde_json::from_str::<OptionalServerConfig>(&std::fs::read_to_string(&config_path)?)?
			.build_default()
	} else {
		ServerConfig::default()
	};

	println!("starting server with config: {config:#?}");

	let server = Server::new(config).await?;

	server.run().await?;

	Ok(())
}
