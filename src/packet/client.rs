use half::f16;

/// enum for a packet which can be received by the client
#[derive(Debug, Clone)]
pub enum ClientPacket {
	/// packet sent by a client to identify itself to the server
	PlayerIdentification {
		/// should always be 0x07 for classic clients >= 0.28
		protocol_version: u8,
		username: String,
		/// currently unverified, original minecraft auth for classic is gone anyway
		/// TODO: use verification key field as password protection? investigate
		verification_key: String,
		_unused: u8,
	},
	/// packet sent when a client changes a block
	/// because changes are reflected immediately, to restrict changes, server must send back its own SetBlock packet with the original block
	SetBlock {
		x: i16,
		y: i16,
		z: i16,
		/// 0x00 for destroy, 0x01 for create
		mode: u8,
		block_type: u8,
	},
	/// sent to update the player's current position and orientation with the server
	PositionOrientation {
		/// should always be 0xff (-1), referring to the player who sent it
		_player_id: i8,
		x: f16,
		y: f16,
		z: f16,
		yaw: u8,
		pitch: u8,
	},
	/// packet for the client to send chat messages
	Message {
		/// should always be 0xff (-1), referring to the player who sent it
		player_id: i8,
		message: String,
	},
}

impl ClientPacket {
	// unused currently, so disabled
	// /// gets the packet's id
	// pub fn get_id(&self) -> u8 {
	// 	match self {
	// 		Self::PlayerIdentification { .. } => 0x00,
	// 		Self::SetBlock { .. } => 0x05,
	// 		Self::PositionOrientation { .. } => 0x08,
	// 		Self::Message { .. } => 0x0d,
	// 	}
	// }

	/// reads the packet
	pub fn read(id: u8, packet: &mut super::PacketReader) -> Option<Self> {
		Some(match id {
			0x00 => Self::PlayerIdentification {
				protocol_version: packet.next_u8()?,
				username: packet.next_string()?,
				verification_key: packet.next_string()?,
				_unused: packet.next_u8()?,
			},
			0x05 => Self::SetBlock {
				x: packet.next_i16()?,
				y: packet.next_i16()?,
				z: packet.next_i16()?,
				mode: packet.next_u8()?,
				block_type: packet.next_u8()?,
			},
			0x08 => Self::PositionOrientation {
				_player_id: packet.next_i8()?,
				x: packet.next_f16()?,
				y: packet.next_f16()?,
				z: packet.next_f16()?,
				yaw: packet.next_u8()?,
				pitch: packet.next_u8()?,
			},
			0x0d => Self::Message {
				player_id: packet.next_i8()?,
				message: packet.next_string()?,
			},
			_ => return None,
		})
	}

	// only needed on the client, so disabled for now
	// /// writes the packet
	// pub fn write(&self, writer: super::PacketWriter) -> super::PacketWriter {
	// 	match self {
	// 		Self::PlayerIdentification {
	// 			protocol_version,
	// 			username,
	// 			verification_key,
	// 			_unused,
	// 		} => writer
	// 			.write_u8(*protocol_version)
	// 			.write_string(username)
	// 			.write_string(verification_key)
	// 			.write_u8(*_unused),
	// 		Self::SetBlock {
	// 			x,
	// 			y,
	// 			z,
	// 			mode,
	// 			block_type,
	// 		} => writer
	// 			.write_i16(*x)
	// 			.write_i16(*y)
	// 			.write_i16(*z)
	// 			.write_u8(*mode)
	// 			.write_u8(*block_type),
	// 		Self::PositionOrientation {
	// 			player_id,
	// 			x,
	// 			y,
	// 			z,
	// 			yaw,
	// 			pitch,
	// 		} => writer
	// 			.write_i8(*player_id)
	// 			.write_f16(*x)
	// 			.write_f16(*y)
	// 			.write_f16(*z)
	// 			.write_u8(*yaw)
	// 			.write_u8(*pitch),
	// 		Self::Message { player_id, message } => {
	// 			writer.write_i8(*player_id).write_string(message)
	// 		}
	// 	}
	// }
}
