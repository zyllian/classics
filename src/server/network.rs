use std::{collections::VecDeque, io::Write, net::SocketAddr, sync::Arc};

use bytes::BytesMut;
use flate2::{write::GzEncoder, Compression};
use half::f16;
use tokio::{
	io::{AsyncReadExt, AsyncWriteExt, Interest},
	net::TcpStream,
	sync::RwLock,
};

use crate::{
	command::{Command, COMMANDS_LIST},
	level::{block::BLOCK_INFO, BlockUpdate, Level},
	packet::{
		client::ClientPacket, server::ServerPacket, PacketWriter, ARRAY_LENGTH, STRING_LENGTH,
	},
	player::{Player, PlayerType},
	server::config::ServerProtectionMode,
};

use super::ServerData;

pub(super) async fn handle_stream(
	mut stream: TcpStream,
	addr: SocketAddr,
	data: Arc<RwLock<ServerData>>,
) -> std::io::Result<()> {
	let mut own_id: i8 = -1;
	let r = handle_stream_inner(&mut stream, addr, data.clone(), &mut own_id).await;

	match r {
		Ok(disconnect_reason) => {
			if let Some(disconnect_reason) = disconnect_reason {
				let packet = ServerPacket::DisconnectPlayer { disconnect_reason };
				let writer = PacketWriter::default().write_u8(packet.get_id());
				let msg = packet.write(writer).into_raw_packet();
				if let Err(e) = stream.write_all(&msg).await {
					eprintln!("Failed to write disconnect packet for <{addr}>: {e}");
				}
			}
		}
		Err(e) => eprintln!("Error in stream handler for <{addr}>: {e}"),
	}

	if let Err(e) = stream.shutdown().await {
		eprintln!("Failed to properly shut down stream for <{addr}>: {e}");
	}

	let mut data = data.write().await;
	if let Some(index) = data.players.iter().position(|p| p.id == own_id) {
		let player = data.players.remove(index);
		data.free_player_ids.push(player.id);

		let despawn_packet = ServerPacket::DespawnPlayer { player_id: own_id };
		let message_packet = ServerPacket::Message {
			player_id: own_id,
			message: format!("&e{} has left the server.", player.username),
		};
		for player in &mut data.players {
			player.packets_to_send.push(despawn_packet.clone());
			player.packets_to_send.push(message_packet.clone());
		}
	}

	Ok(())
}

async fn handle_stream_inner(
	stream: &mut TcpStream,
	addr: SocketAddr,
	data: Arc<RwLock<ServerData>>,
	own_id: &mut i8,
) -> std::io::Result<Option<String>> {
	let mut reply_queue: VecDeque<ServerPacket> = VecDeque::new();
	let mut read_buf;
	let mut id_buf;

	macro_rules! msg {
		($message:expr) => {
			reply_queue.push_back(ServerPacket::Message {
				player_id: -1,
				message: $message,
			});
		};
	}

	macro_rules! spread_packet {
		($data:expr, $packet:expr) => {
			let packet = $packet;
			for player in &mut $data.players {
				player.packets_to_send.push(packet.clone());
			}
		};
	}

	loop {
		if let Some(player) = data.read().await.players.iter().find(|p| p.id == *own_id) {
			if let Some(msg) = &player.should_be_kicked {
				return Ok(Some(format!("Kicked: {msg}")));
			}
		}

		let ready = stream
			.ready(Interest::READABLE | Interest::WRITABLE)
			.await?;

		if ready.is_read_closed() {
			println!("disconnecting {addr}");
			break;
		}

		if ready.is_readable() {
			id_buf = [0u8];
			match stream.try_read(&mut id_buf) {
				Ok(n) => {
					if n == 1 {
						if let Some(size) = ClientPacket::get_size_from_id(id_buf[0]) {
							read_buf = BytesMut::zeroed(size);

							stream.read_exact(&mut read_buf).await?;

							match ClientPacket::read(id_buf[0], &mut read_buf)
								.expect("should never fail: id already checked")
							{
								ClientPacket::PlayerIdentification {
									protocol_version,
									username,
									verification_key,
									_unused,
								} => {
									if protocol_version != 0x07 {
										return Ok(Some("Unknown protocol version! Please connect with a classic 0.30-compatible client.".to_string()));
									}

									let zero = f16::from_f32(0.0);

									let mut data = data.write().await;

									match &data.config.protection_mode {
										ServerProtectionMode::None => {}
										ServerProtectionMode::Password(password) => {
											if verification_key != *password {
												return Ok(Some("Incorrect password!".to_string()));
											}
										}
										ServerProtectionMode::PasswordsByUser(passwords) => {
											if !passwords
												.get(&username)
												.map(|password| verification_key == *password)
												.unwrap_or_default()
											{
												return Ok(Some("Incorrect password!".to_string()));
											}
										}
									}

									for player in &data.players {
										if player.username == username {
											return Ok(Some(
												"Player with username already connected!"
													.to_string(),
											));
										}
									}

									*own_id = data
										.free_player_ids
										.pop()
										.unwrap_or_else(|| data.players.len() as i8);

									let player_type = data
										.config
										.player_perms
										.get(&username)
										.copied()
										.unwrap_or_default();

									let player = Player {
										_addr: addr,
										id: *own_id, // TODO: actually assign user ids
										username,
										// TODO: properly assign spawn stuff
										x: zero,
										y: zero,
										z: zero,
										yaw: 0,
										pitch: 0,
										permissions: player_type,
										packets_to_send: Vec::new(),
										should_be_kicked: None,
									};

									reply_queue.push_back(ServerPacket::ServerIdentification {
										protocol_version: 0x07,
										server_name: data.config.name.clone(),
										server_motd: data.config.motd.clone(),
										user_type: player_type,
									});

									println!("generating level packets");
									reply_queue
										.extend(build_level_packets(&data.level).into_iter());

									let username = player.username.clone();
									data.players.push(player);

									let (spawn_x, spawn_y, spawn_z) =
										if let Some(spawn) = &data.config.spawn {
											(spawn.x, spawn.y, spawn.z)
										} else {
											(16, data.level.y_size / 2 + 2, 16)
										};

									let spawn_packet = ServerPacket::SpawnPlayer {
										player_id: *own_id,
										player_name: username.clone(),
										x: f16::from_f32(spawn_x as f32 + 0.5),
										y: f16::from_f32(spawn_y as f32),
										z: f16::from_f32(spawn_z as f32 + 0.5),
										yaw: 0,
										pitch: 0,
									};
									let message_packet = ServerPacket::Message {
										player_id: *own_id,
										message: format!("&e{} has joined the server.", username),
									};
									for player in &mut data.players {
										player.packets_to_send.push(spawn_packet.clone());
										if player.id != *own_id {
											reply_queue.push_back(ServerPacket::SpawnPlayer {
												player_id: player.id,
												player_name: player.username.clone(),
												x: player.x,
												y: player.y,
												z: player.z,
												yaw: player.yaw,
												pitch: player.pitch,
											});
											player.packets_to_send.push(message_packet.clone());
										}
									}
									msg!("&dWelcome to the server! Enjoyyyyyy".to_string());
									reply_queue.push_back(ServerPacket::UpdateUserType {
										user_type: PlayerType::Operator,
									});
								}
								ClientPacket::SetBlock {
									x,
									y,
									z,
									mode,
									block_type,
								} => {
									let block_type = if mode == 0x00 { 0 } else { block_type };
									let mut data = data.write().await;

									// kick players if they attempt to place a block out of bounds
									if x.clamp(0, data.level.x_size as i16 - 1) != x
										|| y.clamp(0, data.level.y_size as i16 - 1) != y
										|| z.clamp(0, data.level.z_size as i16 - 1) != z
									{
										return Ok(Some(
											"Attempt to place block out of bounds".to_string(),
										));
									}

									let new_block_info = BLOCK_INFO.get(&block_type);
									if new_block_info.is_none() {
										msg!(format!("&cUnknown block ID: 0x{:0x}", block_type));
										continue;
									}
									let new_block_info = new_block_info.expect("will never fail");
									let mut cancel = false;
									let block =
										data.level.get_block(x as usize, y as usize, z as usize);
									let block_info = BLOCK_INFO
										.get(&block)
										.expect("missing block information for block!");

									// check if player has ability to place/break these blocks
									let player_type = data
										.players
										.iter()
										.find_map(|p| (p.id == *own_id).then_some(p.permissions))
										.unwrap_or_default();
									if player_type < new_block_info.place_permissions {
										cancel = true;
										msg!("&cNot allow to place this block.".to_string());
									} else if player_type < block_info.break_permissions {
										cancel = true;
										msg!("&cNot allowed to break this block.".to_string());
									}

									if cancel {
										reply_queue.push_back(ServerPacket::SetBlock {
											x,
											y,
											z,
											block_type: block,
										});
										continue;
									}
									let (x, y, z) = (x as usize, y as usize, z as usize);
									let index = data.level.index(x, y, z);
									data.level.updates.push(BlockUpdate {
										index,
										block: block_type,
									});
									if new_block_info.block_type.needs_update_on_place() {
										data.level.awaiting_update.insert(index);
									}
								}
								ClientPacket::PositionOrientation {
									_player_id: _,
									x,
									y,
									z,
									yaw,
									pitch,
								} => {
									let mut data = data.write().await;
									spread_packet!(
										data,
										ServerPacket::SetPositionOrientation {
											player_id: *own_id,
											x,
											y,
											z,
											yaw,
											pitch,
										}
									);
								}
								ClientPacket::Message { player_id, message } => {
									let mut data = data.write().await;

									if let Some(message) = message.strip_prefix(Command::PREFIX) {
										match Command::parse(message) {
											Ok(cmd) => {
												let player = data
													.players
													.iter()
													.find(|p| p.id == *own_id)
													.expect("missing player");

												if cmd.perms_required() > player.permissions {
													msg!("&cPermissions do not allow you to use this command".to_string());
													continue;
												}

												match cmd {
													Command::Me { action } => {
														let message = format!(
															"&f*{} {action}",
															data.players
																.iter()
																.find(|p| p.id == *own_id)
																.expect("missing player")
																.username
														);
														spread_packet!(
															data,
															ServerPacket::Message {
																player_id,
																message,
															}
														);
													}

													Command::Say { message } => {
														let message =
															format!("&d[SERVER] &f{message}");
														spread_packet!(
															data,
															ServerPacket::Message {
																player_id,
																message,
															}
														);
													}

													Command::SetPermissions {
														player_username,
														permissions,
													} => {
														let player_perms = player.permissions;
														if player_username == player.username {
															msg!("&cCannot change your own permissions".to_string());
															continue;
														} else if permissions >= player_perms {
															msg!("&cCannot set permissions higher or equal to your own".to_string());
															continue;
														}

														let perm_string =
															serde_json::to_string(&permissions)
																.expect("should never fail");

														if let Some(current) = data
															.config
															.player_perms
															.get(player_username)
														{
															if *current >= player_perms {
																msg!("&cThis player outranks or is the same rank as you"
																.to_string());
																continue;
															}
														}

														data.config_needs_saving = true;

														if matches!(permissions, PlayerType::Normal)
														{
															data.config
																.player_perms
																.remove(player_username);
														} else {
															data.config.player_perms.insert(
																player_username.to_string(),
																permissions,
															);
														}
														if let Some(p) = data
															.players
															.iter_mut()
															.find(|p| p.username == player_username)
														{
															p.permissions = permissions;
															p.packets_to_send.push(
																ServerPacket::UpdateUserType {
																	user_type: p.permissions,
																},
															);
															p.packets_to_send.push(ServerPacket::Message {
																player_id: p.id,
																message: format!("Your permissions have been set to {perm_string}")
															});
														}
														msg!(format!("Set permissions for {player_username} to {perm_string}"));
													}
													Command::Kick { username, message } => {
														let player_perms = player.permissions;

														if let Some(other_player) = data
															.players
															.iter_mut()
															.find(|p| p.username == username)
														{
															if player_perms
																<= other_player.permissions
															{
																msg!("&cThis player outranks or is the same rank as you".to_string());
																continue;
															}

															other_player.should_be_kicked = Some(
																message
																	.unwrap_or("<no message>")
																	.to_string(),
															);
															msg!(format!(
																"{} has been kicked",
																other_player.username
															));
														} else {
															msg!(
																"&cPlayer not connected to server!"
																	.to_string()
															);
														}
													}

													Command::Stop => {
														data.stop = true;
													}

													Command::Help { command } => {
														let messages =
															if let Some(command) = command {
																Command::help(command)
															} else {
																let mut messages = vec![
																	"Commands available to you:"
																		.to_string(),
																];
																let mut current_message =
																	"&f".to_string();
																for command in COMMANDS_LIST.iter()
																{
																	if Command::perms_required_by_name(command) > player.permissions {
																	continue;
																}
																	if current_message.len()
																		+ 3 + command.len()
																		> STRING_LENGTH
																	{
																		messages.push(format!(
																			"{current_message},"
																		));
																		current_message =
																			"&f".to_string();
																	}
																	if current_message.len() == 2 {
																		current_message = format!("{current_message}{command}");
																	} else {
																		current_message = format!("{current_message}, {command}");
																	}
																}
																if !current_message.is_empty() {
																	messages.push(current_message);
																}
																messages
															};
														for msg in messages {
															msg!(msg);
														}
													}
												}
											}
											Err(msg) => {
												msg!(format!("&c{msg}"));
											}
										}
									} else {
										println!("{message}");
										let message = format!(
											"&f<{}> {message}",
											data.players
												.iter()
												.find(|p| p.id == *own_id)
												.expect("should never fail")
												.username
										);
										spread_packet!(
											data,
											ServerPacket::Message { player_id, message }
										);
									}
								}
							}
						} else {
							println!("unknown packet id: {}", id_buf[0]);
						}
					}
				}
				Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
				Err(e) => return Err(e),
			}
		}

		if ready.is_writable() {
			{
				let mut data = data.write().await;
				if let Some(player) = data.players.iter_mut().find(|p| p.id == *own_id) {
					for mut packet in player.packets_to_send.drain(..) {
						if let Some(id) = packet.get_player_id() {
							if id == *own_id {
								if !packet.should_echo() {
									continue;
								}
								packet.set_player_id(-1);
							}
						}
						reply_queue.push_back(packet);
					}
				}
			}

			while let Some(packet) = reply_queue.pop_front() {
				let writer = PacketWriter::default().write_u8(packet.get_id());
				let msg = packet.write(writer).into_raw_packet();
				stream.write_all(&msg).await?;
			}
		}
	}

	println!("remaining packets: {}", reply_queue.len());

	Ok(None)
}

/// helper to put together packets that need to be sent to send full level data for the given level
fn build_level_packets(level: &Level) -> Vec<ServerPacket> {
	let mut packets: Vec<ServerPacket> = vec![ServerPacket::LevelInitialize {}];

	// TODO: the type conversions in here may be weird idk
	let volume = level.x_size * level.y_size * level.z_size;
	let mut data = Vec::with_capacity(volume + 4);
	data.extend_from_slice(&(volume as i32).to_be_bytes());
	data.extend_from_slice(&level.blocks);

	let mut e = GzEncoder::new(Vec::new(), Compression::best());
	e.write_all(&data).expect("failed to gzip level data");
	let data = e.finish().expect("failed to gzip level data");
	let data_len = data.len();
	let mut total_bytes = 0;

	for chunk in data.chunks(ARRAY_LENGTH) {
		let chunk_len = chunk.len();
		let percent_complete = (total_bytes * 100 / data_len) as u8;
		packets.push(ServerPacket::LevelDataChunk {
			chunk_length: chunk_len as i16,
			chunk_data: chunk.to_vec(),
			percent_complete,
		});

		total_bytes += chunk_len;
	}

	packets.push(ServerPacket::LevelFinalize {
		x_size: level.x_size as i16,
		y_size: level.y_size as i16,
		z_size: level.z_size as i16,
	});

	packets
}
