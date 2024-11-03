use std::{collections::HashMap, fs::Permissions, io::Write, path::{Path, PathBuf}};

use arg::Args;
use once_cell::sync::OnceCell;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use statejson::StateResponse;

mod statejson;

const ERR_INVALID_ARGS:   i32 = 1;
const ERR_COMMAND_FAILED: i32 = 2;

#[derive(Debug, Args, Clone)]
struct BaseArgs {
	#[arg(short = "p", long = "delay")]
	delay: Option<String>,
	#[arg(short = "s", long = "server", default_value = "\"localhost\".to_string()")]
	server_addr: String,
	#[arg(short = "c", long = "script")]
	// Only relevant for commands with output.
	// if true use parser-friendly output,
	// otherwise output in a human-friendly way.
	script_mode: bool,
}


#[derive(Debug, Args, Clone)]
struct VideoChangeRequestArgs {
	#[arg(short = "v", long = "video")]
	video: Option<String>, 
	#[arg(short = "l", long = "playlist")]
	playlist: Option<String>,

	#[arg(short = "p", long = "delay")]
	delay: Option<String>,
	#[arg(short = "s", long = "server", default_value = "\"localhost\".to_string()")]
	server_addr: String,
}


#[derive(Debug, Args, Clone)]
struct SetFloatArgs {
	#[arg(required)]
	target: f32,

	#[arg(short = "p", long = "delay")]
	delay: Option<String>,
	#[arg(short = "s", long = "server", default_value = "\"localhost\".to_string()")]
	server_addr: String,
}

#[derive(Debug, Clone, Args)]
enum Command {
	// Get information about what the player is currently doing, what songs are playing, etc.
	// Once per 5s
	State(BaseArgs),
	// Get a list of all the user's playlists.
	// Once per 30s
	Playlists(BaseArgs),

	// All remaining commands are Twice per 1s

	// Toggles playback state (default command).
	PlayPause(BaseArgs),
	// Unpauses/starts playback (as long as something is currently ready to play).
	Play(BaseArgs),
	// Pauses playback.
	Pause(BaseArgs),
	// Increase volume.
	VolumeUp(BaseArgs),
	// Decrease volume.
	VolumeDown(BaseArgs),
	// Set Volume to a specific percentage between 0 and 100.
	Volume(SetFloatArgs),
	// Mutes playback.
	Mute(BaseArgs),
	// Unmutes playback.
	Unmute(BaseArgs),
	// Seek to a specific number of seconds into the song.
	Seek(SetFloatArgs),
	// Skip to next song.
	Next(BaseArgs),
	// Restart song or go to previous song.
	Previous(BaseArgs),
	// Sets the repeat mode to none, all songs in queue, or just one song respectively.
	RepeatNone(BaseArgs),
	RepeatAll(BaseArgs),
	RepeatSingle(BaseArgs),
	// Shuffles all songs in the queue.
	Shuffle(BaseArgs),
	// Jump to a specific song in the queue.
	Jumpto(SetFloatArgs),
	// Toggles liking the current song.
	Like(BaseArgs),
	// Toggles disliking the current song.
	Dislike(BaseArgs),
	// Change current song to first parameter and/or start playing the playlist specified by the second.
	// If a playlist is specified, the song must be None or on the playlist or the player will misbehave.
	Open(VideoChangeRequestArgs),
}
impl Command {
	fn get_body(&self) -> String {
		match self {
			Command::State(_)
			| Command::Playlists(_) => String::new(),
			Command::PlayPause(_)      => String::from(r#"{"command":"playPause"}"#),
			Command::Play(_)           => String::from(r#"{"command":"play"}"#),
			Command::Pause(_)          => String::from(r#"{"command":"pause"}"#),
			Command::VolumeUp(_)       => String::from(r#"{"command":"volumeUp"}"#),
			Command::VolumeDown(_)     => String::from(r#"{"command":"volumeDown"}"#),
			Command::Volume(SetFloatArgs { target, .. }) => format!    (r#"{{"command":"setVolume", "data": {}}}"#, target),
			Command::Mute(_)           => String::from(r#"{"command":"mute"}"#),
			Command::Unmute(_)         => String::from(r#"{"command":"unmute"}"#),
			Command::Seek(SetFloatArgs { target, .. })    => format!    (r#"{{"command":"seekTo", "data": {}}}"#, target),
			Command::Next(_)           => String::from(r#"{"command":"next"}"#),
			Command::Previous(_)       => String::from(r#"{"command":"previous"}"#),
			Command::RepeatNone(_)     => String::from(r#"{"command":"repeatMode", "data": 0}"#),
			Command::RepeatAll(_)      => String::from(r#"{"command":"repeatMode", "data": 1}"#),
			Command::RepeatSingle(_)   => String::from(r#"{"command":"repeatMode", "data": 2}"#),
			Command::Shuffle(_)        => String::from(r#"{"command":"shuffle"}"#),
			Command::Jumpto(SetFloatArgs { target, .. }) => format!    (r#"{{"command":"playQueueIndex", "data": {}}}"#, target),
			Command::Like(_)           => String::from(r#"{"command":"toggleLike"}"#),
			Command::Dislike(_)        => String::from(r#"{"command":"toggleDislike"}"#),
			Command::Open(VideoChangeRequestArgs{ video, playlist, .. }) => {
				let video = video.as_ref().map(|s| String::from("\"") + s + "\"").unwrap_or(String::from("null"));
				let playlist = playlist.as_ref().map(|s| String::from("\"") + s + "\"").unwrap_or(String::from("null"));
			 	format!(
					r#"{{"command":"changeVideo", "data": {{ "videoId": {}, "playlistId": {} }} }}"#,
					video,
					playlist
				)
			},
		}
		/*
		format!(r#"{{"command":"{}", "data":{data}}}"#, <<command>>, <<data>>
		 */
	}	
	fn get_path(&self) -> Option<&'static str> {
		Some(match self {
			Command::State(_) => "state",
			Command::Playlists(_) => "playlists",
			_ => return None,
		})
	}
	fn is_get_request(&self) -> bool {
		match self {
			Command::State(_)
			| Command::Playlists(_) => true,
			_ => false,
		}
	}

	fn get_delay(&self) -> Option<&str> {
		match self {
			Command::State(base_args)
			| Command::Playlists(base_args)
			| Command::PlayPause(base_args)
			| Command::Play(base_args)
			| Command::Pause(base_args)
			| Command::VolumeUp(base_args)
			| Command::VolumeDown(base_args)
			| Command::Mute(base_args)
			| Command::Unmute(base_args)
			| Command::Next(base_args)
			| Command::Previous(base_args)
			| Command::RepeatNone(base_args)
			| Command::RepeatAll(base_args)
			| Command::RepeatSingle(base_args)
			| Command::Shuffle(base_args)
			| Command::Like(base_args)
			| Command::Dislike(base_args) => base_args.delay.as_deref(),
			Command::Volume(set_float_args)
			| Command::Seek(set_float_args)
			| Command::Jumpto(set_float_args) => set_float_args.delay.as_deref(),
			Command::Open(video_change_request_args) => video_change_request_args.delay.as_deref(),
		}
	}
	fn get_server_addr(&self) -> &str {
		match self {
			Command::State(base_args)
			| Command::Playlists(base_args)
			| Command::PlayPause(base_args)
			| Command::Play(base_args)
			| Command::Pause(base_args)
			| Command::VolumeUp(base_args)
			| Command::VolumeDown(base_args)
			| Command::Mute(base_args)
			| Command::Unmute(base_args)
			| Command::Next(base_args)
			| Command::Previous(base_args)
			| Command::RepeatNone(base_args)
			| Command::RepeatAll(base_args)
			| Command::RepeatSingle(base_args)
			| Command::Shuffle(base_args)
			| Command::Like(base_args)
			| Command::Dislike(base_args) => &*base_args.server_addr,
			Command::Volume(set_float_args)
			| Command::Seek(set_float_args)
			| Command::Jumpto(set_float_args) => &*set_float_args.server_addr,
			Command::Open(video_change_request_args) => &*video_change_request_args.server_addr,
		}

	}

	// TODO: make output different for script mode and human mode
	#[allow(dead_code)]
	fn is_script_mode(&self) -> bool {
		match self {
			Command::State(base_args)
			| Command::Playlists(base_args) => base_args.script_mode,
			Command::PlayPause(_)
			| Command::Play(_)
			| Command::Pause(_)
			| Command::VolumeUp(_)
			| Command::VolumeDown(_)
			| Command::Mute(_)
			| Command::Unmute(_)
			| Command::Next(_)
			| Command::Previous(_)
			| Command::RepeatNone(_)
			| Command::RepeatAll(_)
			| Command::RepeatSingle(_)
			| Command::Shuffle(_)
			| Command::Like(_)
			| Command::Dislike(_)
			| Command::Volume(_)
			| Command::Seek(_)
			| Command::Jumpto(_)
			| Command::Open(_) => false,
		}

	}

}
	

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct PlaylistEntry<'a> {
	id: &'a str,
	title: &'a str,
}

fn get_token_store_path() -> &'static Path {
	static PATH: OnceCell<PathBuf> = OnceCell::new();
	PATH.get_or_init(|| env_home::env_home_dir().expect("Unable to locate home directory").join(".config/ytmdctrl.tkn"))
}

fn read_token_store() -> Option<HashMap<String, String>> {
	if let Ok(tknstrs) = std::fs::read_to_string(get_token_store_path()) {
		serde_json::from_str::<HashMap<String, String>>(&tknstrs).ok()
	} else {
		None
	}
}

const USEFUL_HELP:  &'static str = "\
Control the Youtube Music Desktop Player from the CLI or scripts.
Options:
	--delay,  -p     Delays execution by a certain amount of time.
	--server, -s     Sets the ip of the server to connect to.
	                 Default is `localhost`.
	--script_mode    Adjusts output of 'get' commands to be better
	                 for scripts. Currently has no effect.
Commands:
	state:           Current player state.
	playlists:       List all playlists in the user's account.
	play-pause:      Toggle playback.
	play:            Resume/Start playback.
	pause:           Pause playback.
	volume-up:       Increase volume.
	volume-down:     Decrease volume.
	volume <target>: Set volume to <target>%.
	mute:            Mutes playback.
	unmute:          Unmutes playback.
	seek <seconds>:  Seeks to <seconds> into the song.
	next:            Skip to next song in the queue.
	previous:        Restart the current song or go back to the previous song in the queue.
	repeat-none:     Sets the repeat mode to None.
	repeat-all:      Sets the repeat mode to All.
	repeat-single:   Sets the repeat mode to One.
	shuffle:         Shuffles the queue (cannot be undone).
	jumpto <index>:  Jumps to a specific <index> in the queue.
	like:            Toggles the liked status of the song.
	dislike:         Toggles the disliked status of the song.
	open
		[--video <video>]
		[--playlist <playlist>]: 
	                 Changes playback to the specified song or playlist. One or both must be specified.
";


#[tokio::main]
async fn main() {
	let mut args: Vec<String> = std::env::args().skip(1).collect();
	// handle standard help command syntax, arg's help command is nonstandard
	if args.iter().find(|a| &**a == "-h" || &**a == "--help").is_some() {
		std::println!("{}", USEFUL_HELP);
		return;
	}
	// if only flags (or nothing) are specified with no command, assume the command is play-pause
	if args.iter().find(|s| !s.starts_with('-')).is_none() {
		args.insert(0, "play-pause".to_owned());
	}
	let command = if let Ok(cmd) = Command::from_args(args.iter().map(|s| &**s)) {
		cmd
	} else {
		// print the help message on invalid commands rather than an unhelpful error
		match &*(args.iter().find(|s| !s.starts_with('-')).unwrap().to_lowercase()) {
			"volume" => std::eprintln!("`volume` requires a percentage to set volume to between 0 and 100\n"),
			"seek" => std::eprintln!("`seek` requires a time to seek to in seconds\n"),
			"jumpto" => std::eprintln!("`jumpto` requires an integer index in the queue to jump to\n"),
			arg => std::eprintln!("Invalid command `{arg}`\n"),
		}
		
		std::println!("{}", USEFUL_HELP);
		std::process::exit(ERR_INVALID_ARGS);
	};
	if let Command::Open(VideoChangeRequestArgs { video: None, playlist: None, ..}) = command {
		eprintln!("`open` requires either --video or --playlist to be specified");
		return;
	}
	let client = reqwest::Client::new();
	// Check for token in store
	let mut store = read_token_store().unwrap_or_else(|| {
		std::fs::create_dir_all(get_token_store_path().parent().unwrap()).unwrap();
		let tkn_file = std::fs::File::create(get_token_store_path()).unwrap();
		tkn_file.set_permissions(owner_only()).unwrap();
		HashMap::new()
	});
	if let Some(token) = read_token_store().and_then(|mut tkstr| tkstr.remove(command.get_server_addr())) {
		main_logic(command, client, &token).await;
		return
	}
	let ip = command.get_server_addr();
	// No token stored, we need to obtain one
	// Get the code from YTMD for requesting authorization
	let code_response = client.post(format!("http://{ip}:9863/api/v1/auth/requestcode")).body(r#"{
		"appId": "ytmdctrl", 
		"appName": "Seta's YTMD CLI", 
		"appVersion": "0.0.2"
	}"#).header("content-type", "application/json").send().await.unwrap();
	if code_response.status() != StatusCode::OK {
		eprintln!("Failed to get code for token request; Enable companion authorization in YTMD settings and rerun command");
		return;
	}
	let code: String = serde_json::from_str::<Value>(
		&code_response.text().await.unwrap()
	).unwrap()["code"].as_str().unwrap().to_string();
	eprintln!("ytmdctrl is not authorized as a companion - please accept the authorization request");
	eprintln!("authorization code is {code}");
	// Use the code to request a token; user will need to have enabled companion authorization and approve 
	// the authorization request
	let token_response = client.post(format!("http://{ip}:9863/api/v1/auth/request")).body(format!(r#"{{
		"appId": "ytmdctrl",
		"code": "{code}"
	}}"#)).header("content-type", "application/json").send().await.unwrap();
	if token_response.status() != StatusCode::OK {
		eprintln!("Failed to get token; Companion Authorization Request Denied");
		return;
	}
	let token: String = serde_json::from_str::<Value>(
		&token_response.text().await.unwrap()
	).unwrap()["token"].as_str().unwrap().to_string();

	store.insert(ip.to_string(), token.clone());
	let mut tkn_file = std::fs::File::create(get_token_store_path()).unwrap();
	if main_logic(command, client, &token).await {
		tkn_file.write(&serde_json::to_vec(&store).unwrap()).unwrap();
	}
}


// Returns `true` if the token was valid. `false` means the token should not be stored.
async fn main_logic(command: Command, client: reqwest::Client, token: &str) -> bool {
	let token = token.trim();
	if let Some(delay) = command.get_delay() {
		let sleep_time = parse_duration::parse(delay).unwrap();
		tokio::time::sleep(sleep_time).await;
	}
	let response = if let Some(path) = command.get_path() {
		client.get(format!("http://{}:9863/api/v1/{}", command.get_server_addr(), path))
			.header("Authorization", token)
			.send().await.unwrap()
	} else {
		let builder = client.post(format!("http://{}:9863/api/v1/command", command.get_server_addr()))
			.header("content-type", "application/json")
			.header("Authorization", token);
		builder
			.body(command.get_body())
			.send().await.unwrap()
	};

	if response.status() == StatusCode::TOO_MANY_REQUESTS {
		eprintln!("Rate limit exceeded");
		eprintln!("Wait {} seconds before submitting another request", 
			response.headers().get("x-ratelimit-reset").and_then(|v| v.to_str().ok()).unwrap_or("5")
		);
		return true;
	} else if !response.status().is_success() {
		eprintln!("Command sent to YTMD Failed: {response:#?}");
		let body = response.text().await.unwrap();
		if let Ok(parsed) = serde_json::from_str::<Value>(&body) {
			if parsed.get("error").map_or(false, |e| e.as_str().map_or(false, |e| e == "UNAUTHORIZED")) {
				// UNAUTHORIZED means our current token is invalid
				eprintln!("Server says token is unauthorized, deleting token.");
				eprintln!("ytmdctrl will need to reauthorize on next run");
				let mut tkn_file = std::fs::File::create(get_token_store_path()).unwrap();
				if let Some(mut store) = read_token_store() {
					store.remove(command.get_server_addr());
					tkn_file.write(&serde_json::to_vec(&store).unwrap()).unwrap();
				}
				return false;
			} else {
				eprintln!("-- Response Body --");
				eprintln!("{}", serde_json::to_string_pretty(&parsed).unwrap())
			}
		} else {
			eprintln!("-- Response Body (failed to parse json, unformatted) --");
			eprintln!("{body}");
		}
		std::process::exit(ERR_COMMAND_FAILED)
	} else if command.is_get_request() {
		let body = response.text().await.unwrap();
		// attempt to parse the response as json so we can pretty print it
		// if that fails, fallback to printing raw text
		match command {
			Command::State(_) => {
				if let Ok(state) = serde_json::from_str::<StateResponse>(&*body) {
					// if command.is_script_mode() {
						println!("Status: {:?} {:?}", state.player.track_state, state.video.as_ref().map_or("", |v| v.title));
						println!("Progress: {:?}s/{:?}s", state.player.video_progress, state.video.as_ref().map_or(0.0, |v| v.duration_seconds));
						println!("Volume: {:?}%", state.player.volume);
						if let Some(queue) = &state.player.queue {
							let mut idx = 0;
							println!("Queue:");
							for video in queue.items.iter() {
								print!("<{idx}> {}", video.title);
								if video.selected {
									println!(" <SELECTED>");
								} else {
									println!("");
								}
								idx += 1;
							}
							println!("Automix Queue:");
							for video in &queue.automix_items {
								println!("<{idx}> {}", video.title);
								idx += 1;
							}
						}
						
					// } else {
					// 	todo!()
					// }
				} else if let Ok(parsed) = serde_json::from_str::<Value>(&body) {
					eprintln!("Unexpected response from YTMD -- falling back to unformatted output");
					println!("{}", serde_json::to_string_pretty(&parsed).unwrap())
				} else {
					eprintln!("Unexpected response from YTMD -- falling back to raw output");
					println!("{body}");
				}
			},
			Command::Playlists(_) => {
				if let Ok(playlists) = serde_json::from_str::<Vec<PlaylistEntry>>(&*body) {
					for pl in playlists {
						println!("{} -> {}", pl.title, pl.id);
					}
				} else if let Ok(parsed) = serde_json::from_str::<Value>(&body) {
					eprintln!("Unexpected response from YTMD -- falling back to unformatted output");
					println!("{}", serde_json::to_string_pretty(&parsed).unwrap())
				} else {
					eprintln!("Unexpected response from YTMD -- falling back to raw output");
					println!("{body}");
				}
			},
			_ => {
				if let Ok(parsed) = serde_json::from_str::<Value>(&body) {
				println!("{}", serde_json::to_string_pretty(&parsed).unwrap())
				} else {
					println!("{body}");
				}
			}
		}
	}
	return true;
}

//TODO: Support non-unix operating systems
#[cfg(target_family="unix")]
fn owner_only() -> Permissions {
    use std::os::unix::fs::PermissionsExt as _;
    Permissions::from_mode(0o600)
}