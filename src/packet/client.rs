use half::f16;

use super::{client_extended::ExtendedClientPacket, SafeBufExtension, STRING_LENGTH};

/// enum for a packet which can be received by the client
#[derive(Debug, Clone)]
pub enum ClientPacket {
	/// packet sent by a client to identify itself to the server
	PlayerIdentification {
		/// should always be 0x07 for classic clients >= 0.28
		protocol_version: u8,
		username: String,
		/// used as the password the client sends to the server
		verification_key: String,
		/// unused in vanilla but used to check the magic number for extension support
		magic_number: u8,
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

	// extension packets
	Extended(ExtendedClientPacket),
}

impl ClientPacket {
	/// gets the size of the packet from the given id (minus one byte for the id)
	pub const fn get_size_from_id(id: u8) -> Option<usize> {
		Some(match id {
			0x00 => 1 + STRING_LENGTH + STRING_LENGTH + 1,
			0x05 => 2 + 2 + 2 + 1 + 1,
			0x08 => 1 + 2 + 2 + 2 + 1 + 1,
			0x0d => 1 + STRING_LENGTH,
			_ => return ExtendedClientPacket::get_size_from_id(id),
		})
	}

	/// reads the packet
	pub fn read<B>(id: u8, buf: &mut B) -> Option<Self>
	where
		B: SafeBufExtension,
	{
		Some(match id {
			0x00 => Self::PlayerIdentification {
				protocol_version: buf.try_get_u8().ok()?,
				username: buf.try_get_string().ok()?,
				verification_key: buf.try_get_string().ok()?,
				magic_number: buf.try_get_u8().ok()?,
			},
			0x05 => Self::SetBlock {
				x: buf.try_get_i16().ok()?,
				y: buf.try_get_i16().ok()?,
				z: buf.try_get_i16().ok()?,
				mode: buf.try_get_u8().ok()?,
				block_type: buf.try_get_u8().ok()?,
			},
			0x08 => Self::PositionOrientation {
				_player_id: buf.try_get_i8().ok()?,
				x: buf.try_get_f16().ok()?,
				y: buf.try_get_f16().ok()?,
				z: buf.try_get_f16().ok()?,
				yaw: buf.try_get_u8().ok()?,
				pitch: buf.try_get_u8().ok()?,
			},
			0x0d => Self::Message {
				player_id: buf.try_get_i8().ok()?,
				message: buf.try_get_string().ok()?,
			},

			id => Self::Extended(ExtendedClientPacket::read(id, buf)?),
		})
	}
}
