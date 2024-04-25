use std::collections::BTreeMap;

use optional_struct::optional_struct;
use serde::{Deserialize, Serialize};

use crate::{level::generation::LevelGeneration, player::PlayerType};

/// configuration for the server
#[optional_struct]
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
	/// the server's name
	pub name: String,
	/// the server's motd
	pub motd: String,
	/// the server's protection mode
	#[serde(rename = "password")]
	pub protection_mode: ServerProtectionMode,
	/// map of user permissions
	pub player_perms: BTreeMap<String, PlayerType>,
	/// the level's name
	pub level_name: String,
	/// the level's size
	pub level_size: ConfigCoordinates,
	/// the level's spawn point
	pub spawn: Option<ConfigCoordinatesWithOrientation>,
	/// the method to generate the server's level with
	pub generation: LevelGeneration,
	/// the server should auto save the world every X minutes, 0 to disable
	pub auto_save_minutes: u64,
}

impl OptionalServerConfig {
	/// builds the server config filling with default options
	pub fn build_default(self) -> ServerConfig {
		self.build(Default::default())
	}
}

impl Default for ServerConfig {
	fn default() -> Self {
		Self {
			name: "classic server wowie".to_string(),
			motd: "here's the default server motd".to_string(),
			protection_mode: ServerProtectionMode::None,
			player_perms: Default::default(),
			level_name: "default".to_string(),
			level_size: ConfigCoordinates {
				x: 256,
				y: 64,
				z: 256,
			},
			spawn: None,
			generation: LevelGeneration::Flat(crate::level::generation::FlatPreset::StoneAndGrass),
			auto_save_minutes: 1,
		}
	}
}

/// coordinates as stored in configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigCoordinates {
	/// the X coordinate
	pub x: usize,
	/// the Y coordinate
	pub y: usize,
	/// the Z coordinate
	pub z: usize,
}

/// coordinates stored in config including orientation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigCoordinatesWithOrientation {
	/// the X coordinate
	pub x: f32,
	/// the Y coordinate
	pub y: f32,
	/// the Z coordinate
	pub z: f32,
	/// the orientation's yaw
	pub yaw: u8,
	/// the orientation's pitch
	pub pitch: u8,
}

/// enum for the different kinds of server protection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ServerProtectionMode {
	/// the server is unprotected and anyone can join with any username
	None,
	/// the server requires a password to join, but you can use any username if you know the password
	Password(String),
	/// the server requires a password to join and the password is checked against each username
	PasswordsByUser(BTreeMap<String, String>),
}
