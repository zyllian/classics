use half::f16;

use crate::{
	packet::{server::ServerPacket, ExtBitmask, STRING_LENGTH},
	player::PlayerType,
	server::{
		config::{ConfigCoordinatesWithOrientation, ServerProtectionMode},
		network::set_player_inventory,
		ServerData,
	},
};

const CMD_ME: &str = "me";
const CMD_SAY: &str = "say";
const CMD_SETPERM: &str = "setperm";
const CMD_KICK: &str = "kick";
const CMD_STOP: &str = "stop";
const CMD_HELP: &str = "help";
const CMD_BAN: &str = "ban";
const CMD_ALLOWENTRY: &str = "allowentry";
const CMD_SETPASS: &str = "setpass";
const CMD_SETLEVELSPAWN: &str = "setlevelspawn";
const CMD_WEATHER: &str = "weather";
const CMD_SAVE: &str = "save";
const CMD_TELEPORT: &str = "tp";

const USERNAME_SELF: &str = "@s";

/// list of commands available on the server
pub const COMMANDS_LIST: &[&str] = &[
	CMD_ME,
	CMD_SAY,
	CMD_SETPERM,
	CMD_KICK,
	CMD_STOP,
	CMD_HELP,
	CMD_BAN,
	CMD_ALLOWENTRY,
	CMD_SETPASS,
	CMD_SETLEVELSPAWN,
	CMD_WEATHER,
	CMD_SAVE,
	CMD_TELEPORT,
];

/// enum for possible commands
#[derive(Debug, Clone)]
pub enum Command<'m> {
	/// for rp, prefixes `action` with `*<username>`
	///
	/// i.e. `/me says hello` becomes `*<username> says hello`
	Me { action: &'m str },
	/// sends a message prefixed with `[SERVER]` instead of the player's username
	Say { message: &'m str },
	/// sets permissions for a player
	SetPermissions {
		player_username: &'m str,
		permissions: PlayerType,
	},
	/// kicks a player from the server
	Kick {
		username: &'m str,
		message: Option<&'m str>,
	},
	/// command to stop the server
	Stop,
	/// gets help about the given command, or about all commands if no command is given
	Help { command: Option<&'m str> },
	/// bans a player from the server
	Ban {
		player_username: &'m str,
		message: Option<&'m str>,
	},
	/// allows a player entry into the server
	AllowEntry {
		player_username: &'m str,
		password: Option<&'m str>,
	},
	/// sets the current player's password
	SetPass { password: &'m str },
	/// sets the level spawn to the player's location
	SetLevelSpawn,
	/// changes the levels weather
	Weather { weather_type: &'m str },
	/// saves the current level
	Save,
	/// teleports a player to the given coordinates or player
	Teleport {
		username: &'m str,
		mode: TeleportMode<'m>,
	},
}

#[derive(Debug, Clone)]
pub enum TeleportMode<'m> {
	Coordinates { x: f32, y: f32, z: f32 },
	Player(&'m str),
}

impl<'m> Command<'m> {
	/// the prefix for commands
	pub const PREFIX: char = '/';

	/// parses a command, returning the parsed command or an error to be displayed to the player who sent the command
	pub fn parse(input: &'m str) -> Result<Command, String> {
		let (command_name, mut arguments) = input.split_once(' ').unwrap_or((input, ""));
		Ok(match command_name {
			CMD_ME => Self::Me { action: arguments },
			CMD_SAY => Self::Say { message: arguments },
			CMD_SETPERM => Self::SetPermissions {
				player_username: Self::next_string(&mut arguments)?,
				permissions: arguments
					.trim()
					.try_into()
					.map_err(|_| format!("&cUnknown permissions type: {arguments}"))?,
			},
			CMD_KICK => {
				let username = Self::next_string(&mut arguments)?;
				let message = arguments.trim();
				let message = (!message.is_empty()).then_some(message);
				Self::Kick { username, message }
			}
			CMD_STOP => Self::Stop,
			CMD_HELP => Self::Help {
				command: (!arguments.is_empty()).then_some(arguments),
			},
			CMD_BAN => {
				let player_username = Self::next_string(&mut arguments)?;
				let message = arguments.trim();
				let message = (!message.is_empty()).then_some(message);
				Self::Ban {
					player_username,
					message,
				}
			}
			CMD_ALLOWENTRY => {
				let player_username = Self::next_string(&mut arguments)?;
				let password = arguments.trim();
				let password = (!password.is_empty()).then_some(password);
				Self::AllowEntry {
					player_username,
					password,
				}
			}
			CMD_SETPASS => Self::SetPass {
				password: arguments.trim(),
			},
			CMD_SETLEVELSPAWN => Self::SetLevelSpawn,
			CMD_WEATHER => Self::Weather {
				weather_type: arguments,
			},
			CMD_SAVE => Self::Save,
			CMD_TELEPORT => {
				let username = Self::next_string(&mut arguments)?;
				let mode = if let Ok(x) = Self::next_f32(&mut arguments) {
					TeleportMode::Coordinates {
						x,
						y: Self::next_f32(&mut arguments)?,
						z: Self::next_f32(&mut arguments)?,
					}
				} else {
					TeleportMode::Player(arguments)
				};

				Self::Teleport { username, mode }
			}
			_ => return Err(format!("Unknown command: {command_name}")),
		})
	}

	/// gets the command's name
	pub fn command_name(&self) -> &'static str {
		match self {
			Self::Me { .. } => CMD_ME,
			Self::Say { .. } => CMD_SAY,
			Self::SetPermissions { .. } => CMD_SETPERM,
			Self::Kick { .. } => CMD_KICK,
			Self::Stop => CMD_STOP,
			Self::Help { .. } => CMD_HELP,
			Self::Ban { .. } => CMD_BAN,
			Self::AllowEntry { .. } => CMD_ALLOWENTRY,
			Self::SetPass { .. } => CMD_SETPASS,
			Self::SetLevelSpawn => CMD_SETLEVELSPAWN,
			Self::Weather { .. } => CMD_WEATHER,
			Self::Save => CMD_SAVE,
			Self::Teleport { .. } => CMD_TELEPORT,
		}
	}

	/// checks which permissions are required to run this command
	pub fn perms_required(&self) -> PlayerType {
		Self::perms_required_by_name(self.command_name())
	}

	/// checks which permissions are required to run a command by name
	pub fn perms_required_by_name(cmd: &str) -> PlayerType {
		match cmd {
			CMD_ME => PlayerType::Normal,
			CMD_STOP => PlayerType::Operator,
			CMD_HELP => PlayerType::Normal,
			CMD_SETPASS => PlayerType::Normal,
			_ => PlayerType::Moderator,
		}
	}

	/// gets help about the given command
	pub fn help(cmd: &str) -> Vec<String> {
		let c = |t: &str| format!("&f{}{cmd} {t}", Self::PREFIX);

		match cmd {
			CMD_ME => vec![
				c("<action>"),
				"&fDisplays an action as if you're doing it.".to_string(),
			],
			CMD_SAY => vec![
				c("<message>"),
				"&fSends a message as being from the server.".to_string(),
			],
			CMD_SETPERM => vec![
				c("<username> <permission level>"),
				"&fSets a player's permission level.".to_string(),
			],
			CMD_KICK => vec![
				c("<username> [reason]"),
				"&fKicks a player from the server.".to_string(),
			],
			CMD_STOP => vec![
				c(""),
				"&fStops the server while saving the level.".to_string(),
			],
			CMD_HELP => vec![
				c("[command]"),
				"&fGets a list of commands or help about a command.".to_string(),
			],
			CMD_BAN => vec![
				c("<username> [reason]"),
				"&fBans a player from the server.".to_string(),
			],
			CMD_ALLOWENTRY => vec![
				c("<username>"),
				"&fAllows a player into the server.".to_string(),
			],
			CMD_SETPASS => vec![c("<new password>"), "&fUpdates your password.".to_string()],
			CMD_SETLEVELSPAWN => vec![
				c(""),
				"&fSets the level's spawn to your location.".to_string(),
			],
			CMD_WEATHER => vec![
				c("<weather type>"),
				"&fSets the level's weather.".to_string(),
			],
			CMD_SAVE => vec![c(""), "&fSaves the current level.".to_string()],
			CMD_TELEPORT => vec![
				c("(<username> or <x> <y> <z>"),
				"&fTeleports to the given username or coordinates.".to_string(),
			],
			_ => vec!["&eUnknown command!".to_string()],
		}
	}

	/// gets the next string argument from the command
	fn next_string(args: &mut &'m str) -> Result<&'m str, String> {
		if args.is_empty() {
			return Err("Missing argument".to_string());
		}

		let (start_index, end_index, extra) = if args.starts_with('"') {
			let mut end_index = 1;
			let mut extra = 1;
			while end_index < args.len() {
				if let Some(index) = args[end_index..].find('"') {
					end_index += index;
					if &args[end_index - 1..=end_index - 1] == "\\" {
					} else {
						break;
					}
				} else {
					end_index = args.len();
					extra = 0;
					break;
				}
			}
			(1, end_index, extra)
		} else {
			(0, args.find(' ').unwrap_or(args.len()), 0)
		};

		let result = &args[start_index..end_index];
		*args = args[end_index + extra..].trim();

		Ok(result)
	}

	/// gets the next f32 argument from the command
	fn next_f32(args: &mut &'m str) -> Result<f32, String> {
		let (s, r) = args.split_once(' ').unwrap_or((args, ""));
		let n = s.parse().map_err(|_| "Expected number!".to_string())?;
		*args = r.trim();
		Ok(n)
	}

	/// processes the command >:3
	pub fn process(self, data: &mut ServerData, own_id: i8) -> Vec<String> {
		let mut messages = Vec::new();

		let player = data
			.players
			.iter()
			.find(|p| p.id == own_id)
			.expect("missing player");

		if self.perms_required() > player.permissions {
			messages.push("&cPermissions do not allow you to use this command".to_string());
			return messages;
		}

		match self {
			Command::Me { action } => {
				let message = format!(
					"&f*{} {action}",
					data.players
						.iter()
						.find(|p| p.id == own_id)
						.expect("missing player")
						.username
				);
				data.spread_packet(ServerPacket::Message {
					player_id: own_id,
					message,
				});
			}

			Command::Say { message } => {
				let message = format!("&d[SERVER] &f{message}");
				data.spread_packet(ServerPacket::Message {
					player_id: own_id,
					message,
				});
			}

			Command::SetPermissions {
				player_username,
				permissions,
			} => {
				let player_perms = player.permissions;
				if player_username == player.username {
					messages.push("&cCannot change your own permissions".to_string());
					return messages;
				} else if permissions >= player_perms {
					messages
						.push("&cCannot set permissions higher or equal to your own".to_string());
					return messages;
				}

				let perm_string: &'static str = permissions.into();

				if let Some(current) = data.config.player_perms.get(player_username) {
					if *current >= player_perms {
						messages
							.push("&cThis player outranks or is the same rank as you".to_string());
						return messages;
					}
				}

				data.config_needs_saving = true;

				if matches!(permissions, PlayerType::Normal) {
					data.config.player_perms.remove(player_username);
				} else {
					data.config
						.player_perms
						.insert(player_username.to_string(), permissions);
				}
				if let Some(p) = data
					.players
					.iter_mut()
					.find(|p| p.username == player_username)
				{
					p.permissions = permissions;
					p.packets_to_send.push(ServerPacket::UpdateUserType {
						user_type: p.permissions,
					});
					p.packets_to_send.push(ServerPacket::Message {
						player_id: p.id,
						message: format!("Your permissions have been set to {perm_string}"),
					});

					if p.extensions.contains(ExtBitmask::InventoryOrder) {
						set_player_inventory(
							p.permissions,
							p.extensions,
							p.custom_blocks_support_level,
							&mut p.packets_to_send,
						);
					}
				}
				messages.push(format!(
					"Set permissions for {player_username} to {perm_string}"
				));
			}

			Command::Kick { username, message } => {
				let player_perms = player.permissions;

				if let Some(other_player) = data.players.iter_mut().find(|p| p.username == username)
				{
					if player_perms <= other_player.permissions {
						messages
							.push("&cThis player outranks or is the same rank as you".to_string());
						return messages;
					}

					other_player.should_be_kicked =
						Some(format!("Kicked: {}", message.unwrap_or("<no message>")));
					messages.push(format!("{} has been kicked", other_player.username));
				} else {
					messages.push("&cPlayer not connected to server!".to_string());
				}
			}

			Command::Stop => {
				data.stop = true;
			}

			Command::Help { command } => {
				let msgs = if let Some(command) = command {
					Command::help(command)
				} else {
					let mut msgs = vec!["Commands available to you:".to_string()];
					let mut current_message = "&f".to_string();
					for command in COMMANDS_LIST.iter() {
						if Command::perms_required_by_name(command) > player.permissions {
							continue;
						}
						if current_message.len() + 3 + command.len() > STRING_LENGTH {
							msgs.push(format!("{current_message},"));
							current_message = "&f".to_string();
						}
						if current_message.len() == 2 {
							current_message = format!("{current_message}{command}");
						} else {
							current_message = format!("{current_message}, {command}");
						}
					}
					if !current_message.is_empty() {
						msgs.push(current_message);
					}
					msgs
				};
				for msg in msgs {
					messages.push(msg);
				}
			}

			Command::Ban {
				player_username,
				message,
			} => {
				let player_perms = player.permissions;
				if let ServerProtectionMode::PasswordsByUser(passwords) =
					&mut data.config.protection_mode
				{
					if !passwords.contains_key(player_username) {
						messages.push("&cPlayer is already banned!".to_string());
					} else {
						passwords.remove(player_username);
						data.config.player_perms.remove(player_username);
						data.config_needs_saving = true;
						if let Some(other_player) = data
							.players
							.iter_mut()
							.find(|p| p.username == player_username)
						{
							if player_perms <= other_player.permissions {
								messages.push(
									"&cThis player outranks or is the same rank as you".to_string(),
								);
								return messages;
							}

							other_player.should_be_kicked =
								Some(format!("Banned: {}", message.unwrap_or("<no_message>")));
						}
						messages.push(format!("{} has been banned", player_username));
					}
				} else {
					messages.push("&cServer must be set to per-user passwords!".to_string());
				}
			}

			Command::AllowEntry {
				player_username,
				password,
			} => {
				if let ServerProtectionMode::PasswordsByUser(passwords) =
					&mut data.config.protection_mode
				{
					if passwords.contains_key(player_username) {
						messages.push("&cPlayer is already allowed in the server!".to_string());
					} else {
						let password = password
							.map(|p| p.to_string())
							.unwrap_or_else(|| nanoid::nanoid!());
						messages.push(format!("{player_username} is now allowed in the server."));
						messages.push(format!("Password: {password}"));
						passwords.insert(player_username.to_string(), password);
						data.config_needs_saving = true;
					}
				} else {
					messages.push("&cServer must be set to per-user passwords!".to_string());
				}
			}

			Command::SetPass { password } => {
				let username = player.username.clone();
				if let ServerProtectionMode::PasswordsByUser(passwords) =
					&mut data.config.protection_mode
				{
					passwords.insert(username, password.to_string());
					data.config_needs_saving = true;
					messages.push("Updated password!".to_string());
				} else {
					messages.push("&cServer must be set to per-user passwords!".to_string());
				}
			}

			Command::SetLevelSpawn => {
				data.config.spawn = Some(ConfigCoordinatesWithOrientation {
					x: player.x.to_f32(),
					y: player.y.to_f32(),
					z: player.z.to_f32(),
					yaw: player.yaw,
					pitch: player.pitch,
				});
				data.config_needs_saving = true;
				messages.push("Level spawn updated!".to_string());
			}

			Command::Weather { weather_type } => {
				if let Ok(weather_type) = weather_type.try_into() {
					data.level.weather = weather_type;
					let packet = ServerPacket::EnvWeatherType { weather_type };
					for player in &mut data.players {
						player.packets_to_send.push(packet.clone());
					}
					messages.push("Weather updated!".to_string());
				} else {
					messages.push(format!("&cUnknown weather type {weather_type}!"));
				}
			}

			Command::Save => {
				data.level.save_now = true;
				messages.push("Saving level...".to_string());
			}

			Command::Teleport { username, mode } => {
				let username = if username == USERNAME_SELF {
					player.username.clone()
				} else {
					username.to_string()
				};

				let (x, y, z, yaw, pitch, msg) = match mode {
					TeleportMode::Player(username) => {
						let username = if username == USERNAME_SELF {
							player.username.clone()
						} else {
							username.to_string()
						};
						if let Some(player) =
							data.players.iter_mut().find(|p| p.username == username)
						{
							(
								player.x,
								player.y,
								player.z,
								Some(player.yaw),
								Some(player.pitch),
								Some(format!("You have been teleported to {username}.")),
							)
						} else {
							messages.push(format!("Unknown username: {username}"));
							return messages;
						}
					}
					TeleportMode::Coordinates { x, y, z } => (
						f16::from_f32(x + 0.5),
						f16::from_f32(y + 1.0),
						f16::from_f32(z + 0.5),
						None,
						None,
						None,
					),
				};

				if let Some(player) = data.players.iter_mut().find(|p| p.username == username) {
					player.x = x;
					player.y = y;
					player.z = z;
					let packet = ServerPacket::SetPositionOrientation {
						player_id: player.id,
						x,
						y,
						z,
						yaw: yaw.unwrap_or(player.yaw),
						pitch: pitch.unwrap_or(player.pitch),
					};
					let id = player.id;

					for player in &mut data.players {
						let mut packet = packet.clone();
						if player.id == id {
							packet.set_player_id(-1);
							player.packets_to_send.push(ServerPacket::Message {
								player_id: -1,
								message: msg.clone().unwrap_or_else(|| {
									format!("You have been teleported to {x}, {y}, {z}.")
								}),
							});
						}
						player.packets_to_send.push(packet);
					}
				} else {
					messages.push(format!("&fUnknown username: {username}!"));
				}
			}
		}

		messages
	}
}
