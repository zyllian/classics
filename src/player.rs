use std::net::SocketAddr;

use half::f16;

use crate::packet::server::ServerPacket;

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
	pub player_type: PlayerType,

	/// the player's IP address
	pub _addr: SocketAddr,
	/// queue of packets to be sent to this player
	pub packets_to_send: Vec<ServerPacket>,
}

/// enum describing types of players
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerType {
	/// a normal player
	Normal = 0x00,
	/// a player who's an operator
	Operator = 0x64,
}

impl Default for PlayerType {
	fn default() -> Self {
		Self::Normal
	}
}

impl TryFrom<u8> for PlayerType {
	type Error = ();

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		if value == Self::Normal as u8 {
			Ok(Self::Normal)
		} else if value == Self::Operator as u8 {
			Ok(Self::Operator)
		} else {
			Err(())
		}
	}
}
