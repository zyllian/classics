use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::level::generation::LevelGeneration;

/// configuration for the server
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
	/// the server's name
	pub name: String,
	/// the server's motd
	pub motd: String,
	/// the server's protection mode
	#[serde(rename = "password")]
	pub protection_mode: ServerProtectionMode,
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
			protection_mode: ServerProtectionMode::None,
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

/// enum for the different kinds of server protection
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ServerProtectionMode {
	/// the server is unprotected and anyone can join with any username
	None,
	/// the server requires a password to join, but you can use any username if you know the password
	Password(String),
	/// the server requires a password to join and the password is checked against each username
	PasswordsByUser(BTreeMap<String, String>),
}
