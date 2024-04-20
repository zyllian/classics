#![feature(lazy_cell)]

use std::path::PathBuf;

use server::{config::ServerConfig, Server};

mod level;
mod packet;
mod player;
mod server;

const CONFIG_FILE: &str = "./server-config.json";

#[tokio::main]
async fn main() -> std::io::Result<()> {
	let config_path = PathBuf::from(CONFIG_FILE);
	let config = if config_path.exists() {
		serde_json::from_str(&std::fs::read_to_string(config_path)?)
			.expect("failed to deserialize config")
	} else {
		let config = ServerConfig::default();
		std::fs::write(
			config_path,
			serde_json::to_string_pretty(&config).expect("failed to serialize default config"),
		)?;
		config
	};

	println!("starting server with config: {config:#?}");

	let mut server = Server::new(config).await?;

	server.run().await?;

	Ok(())
}
