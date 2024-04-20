pub mod config;
mod network;

use std::sync::Arc;

use tokio::{net::TcpListener, sync::RwLock};

use crate::{level::Level, player::Player};

use self::config::ServerConfig;

const DEFAULT_SERVER_SIZE: usize = 128;

/// the server
#[derive(Debug)]
pub struct Server {
	/// shared server data
	pub data: Arc<RwLock<ServerData>>,
	/// the server's listener
	pub listener: TcpListener,
}

/// shared server data
#[derive(Debug)]
pub struct ServerData {
	/// the level
	pub level: Level,
	/// list of players connected to the server
	pub players: Vec<Player>,
	/// list of player ids which have been freed up
	pub free_player_ids: Vec<i8>,
	/// the server's config
	pub config: ServerConfig,
}

impl Server {
	/// creates a new server with a generated level
	pub async fn new(config: ServerConfig) -> std::io::Result<Self> {
		println!("generating level");
		let mut rng = rand::thread_rng();
		let (level_x, level_y, level_z) = if let Some(size) = &config.level_size {
			(size.x, size.y, size.z)
		} else {
			(
				DEFAULT_SERVER_SIZE,
				DEFAULT_SERVER_SIZE,
				DEFAULT_SERVER_SIZE,
			)
		};
		let mut level = Level::new(level_x, level_y, level_z);
		config.generation.generate(&mut level, &mut rng);
		println!("done!");

		Self::new_with_level(config, level).await
	}

	/// creates a new server with the given level
	pub async fn new_with_level(config: ServerConfig, level: Level) -> std::io::Result<Self> {
		let listener = TcpListener::bind("0.0.0.0:25565").await?;

		Ok(Self {
			data: Arc::new(RwLock::new(ServerData {
				level,
				players: Default::default(),
				free_player_ids: Vec::new(),
				config,
			})),
			listener,
		})
	}

	/// starts the server
	pub async fn run(&mut self) -> std::io::Result<()> {
		loop {
			let (stream, addr) = self.listener.accept().await?;
			println!("connection from {addr}");
			let data = self.data.clone();
			tokio::spawn(async move {
				network::handle_stream(stream, addr, data)
					.await
					.expect("failed to handle client stream");
			});
		}
	}
}
