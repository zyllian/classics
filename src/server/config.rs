use serde::{Deserialize, Serialize};

use crate::level::generation::LevelGeneration;

/// configuration for the server
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
	/// the server's name
	pub name: String,
	/// the server's motd
	pub motd: String,
	/// the server's password, if any
	pub password: Option<String>,
	/// the level's size
	pub level_size: Option<ConfigCoordinates>,
	/// the level's spawn point
	pub spawn: Option<ConfigCoordinates>,
	/// the method to generate the server's level with
	pub generation: LevelGeneration,
}

impl Default for ServerConfig {
	fn default() -> Self {
		Self {
			name: "classic server wowie".to_string(),
			motd: "here's the default server motd".to_string(),
			password: None,
			level_size: None,
			spawn: None,
			generation: LevelGeneration::Empty,
		}
	}
}

/// coordinates as stored in configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCoordinates {
	/// the X coordinate
	pub x: usize,
	/// the Y coordinate
	pub y: usize,
	/// the Z coordinate
	pub z: usize,
}
