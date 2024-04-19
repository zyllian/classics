use half::f16;

use crate::player::PlayerType;

#[derive(Debug, Clone)]
#[allow(unused)]
pub enum ServerPacket {
	/// packet sent as a response to joining clients
	ServerIdentification {
		/// should be 0x07
		protocol_version: u8,
		server_name: String,
		server_motd: String,
		user_type: PlayerType,
	},
	/// since clients do not notify the server when leaving, the ping packet is used to check if the client is still connected
	/// TODO: implement pinging? classicube works fine without it
	Ping {},
	/// informs clients that there is incoming level data
	LevelInitialize {},
	/// packet to send a chunk (not minecraft chunk) of gzipped level data
	LevelDataChunk {
		chunk_length: i16,
		chunk_data: Vec<u8>,
		percent_complete: u8,
	},
	/// packet sent after chunk data is finished sending containing the level dimensions
	LevelFinalize {
		x_size: i16,
		y_size: i16,
		z_size: i16,
	},

	/// indicates a block change
	/// when a player changes a block, their own change is echoed back to them
	SetBlock {
		x: i16,
		y: i16,
		z: i16,
		block_type: u8,
	},
	/// packet sent when a new player spawns
	/// also contains their spawn point
	SpawnPlayer {
		player_id: i8,
		player_name: String,
		x: f16,
		y: f16,
		z: f16,
		yaw: u8,
		pitch: u8,
	},
	/// packet to set a player's position and orientation
	SetPositionOrientation {
		player_id: i8,
		x: f16,
		y: f16,
		z: f16,
		yaw: u8,
		pitch: u8,
	},
	/// packet to update a player's position and orientation
	/// TODO: implement?
	UpdatePositionOrientation {
		player_id: i8,
		x_change: f16,
		y_change: f16,
		z_change: f16,
		yaw: u8,
		pitch: u8,
	},
	/// packet to update a player's position
	/// TODO: implement?
	UpdatePosition {
		player_id: i8,
		x_change: f16,
		y_change: f16,
		z_change: f16,
	},
	/// packet to update a player's orientation
	/// TODO: implement?
	UpdateOrientation { player_id: i8, yaw: u8, pitch: u8 },
	/// packet sent when a player is despawned from the world (i.e. when leaving)
	DespawnPlayer { player_id: i8 },
	/// packet sent when there's a chat message to go out
	Message { player_id: i8, message: String },
	/// informs a client that they're being disconnected from the server and why
	DisconnectPlayer { disconnect_reason: String },
	/// packet sent to a user to inform them that their user type has changed
	UpdateUserType {
		/// 0x00 for normal, 0x64 for op
		user_type: PlayerType,
	},
}

impl ServerPacket {
	/// gets the packet's id
	pub fn get_id(&self) -> u8 {
		match self {
			Self::ServerIdentification { .. } => 0x00,
			Self::Ping {} => 0x01,
			Self::LevelInitialize {} => 0x02,
			Self::LevelDataChunk { .. } => 0x03,
			Self::LevelFinalize { .. } => 0x04,
			Self::SetBlock { .. } => 0x06,
			Self::SpawnPlayer { .. } => 0x07,
			Self::SetPositionOrientation { .. } => 0x08,
			Self::UpdatePositionOrientation { .. } => 0x09,
			Self::UpdatePosition { .. } => 0x0a,
			Self::UpdateOrientation { .. } => 0x0b,
			Self::DespawnPlayer { .. } => 0x0c,
			Self::Message { .. } => 0x0d,
			Self::DisconnectPlayer { .. } => 0x0e,
			Self::UpdateUserType { .. } => 0x0f,
		}
	}

	/// writes the packet
	pub fn write(&self, writer: super::PacketWriter) -> super::PacketWriter {
		match self {
			Self::ServerIdentification {
				protocol_version,
				server_name,
				server_motd,
				user_type,
			} => writer
				.write_u8(*protocol_version)
				.write_string(server_name)
				.write_string(server_motd)
				.write_u8(*user_type as u8),
			Self::Ping {} => writer,
			Self::LevelInitialize {} => writer,
			Self::LevelDataChunk {
				chunk_length,
				chunk_data,
				percent_complete,
			} => writer
				.write_i16(*chunk_length)
				.write_array(chunk_data)
				.write_u8(*percent_complete),
			Self::LevelFinalize {
				x_size,
				y_size,
				z_size,
			} => writer
				.write_i16(*x_size)
				.write_i16(*y_size)
				.write_i16(*z_size),
			Self::SetBlock {
				x,
				y,
				z,
				block_type,
			} => writer
				.write_i16(*x)
				.write_i16(*y)
				.write_i16(*z)
				.write_u8(*block_type),
			Self::SpawnPlayer {
				player_id,
				player_name,
				x,
				y,
				z,
				yaw,
				pitch,
			} => writer
				.write_i8(*player_id)
				.write_string(player_name)
				.write_f16(*x)
				.write_f16(*y)
				.write_f16(*z)
				.write_u8(*yaw)
				.write_u8(*pitch),
			Self::SetPositionOrientation {
				player_id,
				x,
				y,
				z,
				yaw,
				pitch,
			} => writer
				.write_i8(*player_id)
				.write_f16(*x)
				.write_f16(*y)
				.write_f16(*z)
				.write_u8(*yaw)
				.write_u8(*pitch),
			Self::UpdatePositionOrientation {
				player_id,
				x_change,
				y_change,
				z_change,
				yaw,
				pitch,
			} => writer
				.write_i8(*player_id)
				.write_f16(*x_change)
				.write_f16(*y_change)
				.write_f16(*z_change)
				.write_u8(*yaw)
				.write_u8(*pitch),
			Self::UpdatePosition {
				player_id,
				x_change,
				y_change,
				z_change,
			} => writer
				.write_i8(*player_id)
				.write_f16(*x_change)
				.write_f16(*y_change)
				.write_f16(*z_change),
			Self::UpdateOrientation {
				player_id,
				yaw,
				pitch,
			} => writer.write_i8(*player_id).write_u8(*yaw).write_u8(*pitch),
			Self::DespawnPlayer { player_id } => writer.write_i8(*player_id),
			Self::Message { player_id, message } => {
				writer.write_i8(*player_id).write_string(message)
			}
			Self::DisconnectPlayer { disconnect_reason } => writer.write_string(disconnect_reason),
			Self::UpdateUserType { user_type } => writer.write_u8(*user_type as u8),
		}
	}

	/// gets the player id contained in the packet, if any
	pub fn get_player_id(&self) -> Option<i8> {
		Some(match self {
			Self::SpawnPlayer { player_id, .. } => *player_id,
			Self::SetPositionOrientation { player_id, .. } => *player_id,
			Self::UpdatePositionOrientation { player_id, .. } => *player_id,
			Self::UpdatePosition { player_id, .. } => *player_id,
			Self::UpdateOrientation { player_id, .. } => *player_id,
			Self::DespawnPlayer { player_id, .. } => *player_id,
			Self::Message { player_id, .. } => *player_id,
			_ => return None,
		})
	}

	/// sets the player id in the packet if possible
	pub fn set_player_id(&mut self, new_player_id: i8) {
		match self {
			Self::SpawnPlayer { player_id, .. } => *player_id = new_player_id,
			Self::SetPositionOrientation { player_id, .. } => *player_id = new_player_id,
			Self::UpdatePositionOrientation { player_id, .. } => *player_id = new_player_id,
			Self::UpdatePosition { player_id, .. } => *player_id = new_player_id,
			Self::UpdateOrientation { player_id, .. } => *player_id = new_player_id,
			Self::DespawnPlayer { player_id, .. } => *player_id = new_player_id,
			Self::Message { player_id, .. } => *player_id = new_player_id,
			_ => {}
		}
	}

	/// gets whether this packet should echo back to the current player
	pub fn should_echo(&self) -> bool {
		matches!(
			self,
			Self::SetBlock { .. } | Self::SpawnPlayer { .. } | Self::Message { .. }
		)
	}
}
