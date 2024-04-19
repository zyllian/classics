mod network;

use std::sync::Arc;

// use parking_lot::RwLock;
use rand::Rng;
use tokio::{net::TcpListener, sync::RwLock};

use crate::{level::Level, player::Player};

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
}

impl Server {
	/// creates a new server with a generated level
	pub async fn new() -> std::io::Result<Self> {
		println!("generating level");
		let mut rng = rand::thread_rng();
		let mut level = Level::new(
			DEFAULT_SERVER_SIZE,
			DEFAULT_SERVER_SIZE,
			DEFAULT_SERVER_SIZE,
		);
		for x in 0..level.x_size {
			for y in 0..(level.y_size / 2) {
				for z in 0..level.z_size {
					level.set_block(x, y, z, rng.gen_range(0..50));
				}
			}
		}
		println!("done!");

		Self::new_with_level(level).await
	}

	/// creates a new server with the given level
	pub async fn new_with_level(level: Level) -> std::io::Result<Self> {
		let listener = TcpListener::bind("127.0.0.1:25565").await?;

		Ok(Self {
			data: Arc::new(RwLock::new(ServerData {
				level,
				players: Default::default(),
				free_player_ids: Vec::new(),
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
