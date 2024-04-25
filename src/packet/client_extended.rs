use super::{SafeBufExtension, STRING_LENGTH};

/// extended client packets
#[derive(Debug, Clone)]
pub enum ExtendedClientPacket {
	/// packet containing the client name and the number of extensions it supports
	ExtInfo {
		app_name: String,
		extension_count: i16,
	},
	/// packet containing a supported extension name and version
	ExtEntry { ext_name: String, version: i32 },
}

impl ExtendedClientPacket {
	/// gets the size of the packet from the given id (minus one byte for the id)
	pub const fn get_size_from_id(id: u8) -> Option<usize> {
		Some(match id {
			0x10 => STRING_LENGTH + 2,
			0x11 => STRING_LENGTH + 4,
			_ => return None,
		})
	}

	/// reads the packet
	pub fn read<B>(id: u8, buf: &mut B) -> Option<Self>
	where
		B: SafeBufExtension,
	{
		Some(match id {
			0x10 => Self::ExtInfo {
				app_name: buf.try_get_string().ok()?,
				extension_count: buf.try_get_i16().ok()?,
			},
			0x11 => Self::ExtEntry {
				ext_name: buf.try_get_string().ok()?,
				version: buf.try_get_i32().ok()?,
			},
			_ => return None,
		})
	}
}
