use tokio::net::TcpStream;

use internal::{
	error::GeneralError,
	level::block::CUSTOM_BLOCKS_SUPPORT_LEVEL,
	packet::{
		client::ClientPacket, client_extended::ExtendedClientPacket, server::ServerPacket,
		ExtBitmask, ExtInfo,
	},
};

use super::{next_packet, write_packets};

pub async fn get_supported_extensions(
	stream: &mut TcpStream,
) -> Result<(ExtBitmask, u8), GeneralError> {
	let extensions = ExtBitmask::all_bits().all_contained_info();

	write_packets(
		stream,
		Some(ServerPacket::ExtInfo {})
			.into_iter()
			.chain(extensions.iter().map(|info| ServerPacket::ExtEntry {
				ext_name: info.ext_name.to_string(),
				version: info.version,
			})),
	)
	.await?;

	let client_extensions = if let Some(ClientPacket::Extended(ExtendedClientPacket::ExtInfo {
		app_name,
		extension_count,
	})) = next_packet(stream).await?
	{
		println!("client name: {app_name}");
		let mut client_extensions = Vec::with_capacity(extension_count as usize);
		for _ in 0..extension_count {
			if let Some(ClientPacket::Extended(ExtendedClientPacket::ExtEntry {
				ext_name,
				version,
			})) = next_packet(stream).await?
			{
				client_extensions.push(ExtInfo::new(ext_name, version, ExtBitmask::none()));
			} else {
				return Err(GeneralError::Custom(
					"expected ExtEntry packet!".to_string(),
				));
			}
		}
		client_extensions.retain_mut(|cext| {
			if let Some(sext) = extensions
				.iter()
				.find(|sext| sext.ext_name == cext.ext_name && sext.version == cext.version)
			{
				cext.bitmask = sext.bitmask;
				true
			} else {
				false
			}
		});
		client_extensions
	} else {
		Vec::new()
	};

	println!("mutual extensions: {client_extensions:?}");

	let final_bitmask = client_extensions
		.into_iter()
		.fold(ExtBitmask::none(), |acc, ext| acc | ext.bitmask);

	let custom_blocks_support_level = if final_bitmask.contains(ExtBitmask::CustomBlocks) {
		write_packets(
			stream,
			Some(ServerPacket::CustomBlockSupportLevel).into_iter(),
		)
		.await?;
		if let Some(ClientPacket::Extended(ExtendedClientPacket::CustomBlockSupportLevel {
			support_level,
		})) = next_packet(stream).await?
		{
			support_level.min(CUSTOM_BLOCKS_SUPPORT_LEVEL)
		} else {
			return Err(GeneralError::Custom(
				"expected CustomBlockSupportLevel packet!".to_string(),
			));
		}
	} else {
		0
	};

	Ok((final_bitmask, custom_blocks_support_level))
}
