use std::net::SocketAddr;

use half::f16;
use serde::{Deserialize, Serialize};

use crate::packet::{server::ServerPacket, ExtBitmask};

/// struct for players
#[derive(Debug)]
pub struct Player {
	/// the player's id
	pub id: i8,
	/// the player's username
	pub username: String,
	/// the player's X coordinate
	pub x: f16,
	/// the player's Y coordinate
	pub y: f16,
	/// the player's Z coordinate
	pub z: f16,
	/// the player's yaw
	pub yaw: u8,
	/// the player's pitch
	pub pitch: u8,
	/// the player's permission state
	pub permissions: PlayerType,

	/// the player's IP address
	pub _addr: SocketAddr,
	/// the player's supported extensions
	pub extensions: ExtBitmask,
	/// the level of custom blocks this client supports
	pub custom_blocks_support_level: u8,
	/// queue of packets to be sent to this player
	pub packets_to_send: Vec<ServerPacket>,
	/// whether this player should be kicked and the message to give
	pub should_be_kicked: Option<String>,
}

/// enum describing types of players
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Serialize,
	Deserialize,
	strum::EnumString,
	strum::IntoStaticStr,
)]
#[strum(ascii_case_insensitive)]
pub enum PlayerType {
	/// a normal player
	Normal,
	/// moderator of the server
	Moderator,
	/// a player who's an operator
	Operator,
}

impl Default for PlayerType {
	fn default() -> Self {
		Self::Normal
	}
}

impl From<&PlayerType> for u8 {
	fn from(val: &PlayerType) -> Self {
		match val {
			PlayerType::Normal => 0,
			PlayerType::Moderator => 0x64,
			PlayerType::Operator => 0x64,
		}
	}
}
