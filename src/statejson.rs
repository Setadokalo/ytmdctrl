use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr)]
#[repr(i8)]
pub enum PlaybackState {
	Unknown = -1,
	Paused = 0,
	Playing = 1,
	Buffering = 2,
}

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr)]
#[repr(i8)]
pub enum RepeatMode {
	Unknown = -1,
	None = 0,
	All = 1,
	One = 2,
}

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr)]
#[repr(i8)]
pub enum LikeState {
	Unknown = -1,
	Dislike = 0,
	Indifferent = 1,
	Like = 2,
}

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr)]
#[repr(i8)]
pub enum VideoType {
	Unknown = -1,
	Audio = 0,
	Video = 1,
	Uploaded = 2,
	Podcast = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResponse<'a> {
	pub player: PlayerState<'a>,
	pub video: Option<VideoState<'a>>,
	#[serde(rename = "playlistId")]
	pub playlist_id: &'a str
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState<'a> {
	#[serde(rename = "trackState")]
	pub track_state: PlaybackState,
	#[serde(rename = "videoProgress")]
	pub video_progress: f32,
	pub volume: u8,
	#[serde(rename = "adPlaying")]
	pub ad_playing: bool,
	#[serde(borrow)]
	pub queue: Option<QueueState<'a>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueState<'a> {
	pub autoplay: bool,
	#[serde(borrow)]
	pub items: Vec<QueueItemState<'a>>,
	#[serde(rename = "automixItems")]
	pub automix_items: Vec<QueueItemState<'a>>,
	#[serde(rename = "isGenerating")]
	pub is_generating: bool,
	#[serde(rename = "isInfinite")]
	pub is_infinite: bool,
	#[serde(rename = "repeatMode")]
	pub repeat_mode: RepeatMode,
	#[serde(rename = "selectedItemIndex")]
	pub selected_item_index: isize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemState<'a> {
	pub thumbnails: Vec<ThumbnailState<'a>>,
	pub title: &'a str,
	pub author: &'a str,
	pub duration: &'a str,
	pub selected: bool,
	#[serde(rename = "videoId")]
	pub video_id: &'a str,
	pub counterparts: Option<Vec<QueueItemState<'a>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoState<'a> {
	pub author: &'a str,
	#[serde(rename = "channelId")]
	pub channel_id: &'a str,
	pub title: &'a str,
	pub album: Option<&'a str>,
	#[serde(rename = "albumId")]
	pub album_id: Option<&'a str>,
	#[serde(rename = "likeStatus")]
	pub like_status: Option<LikeState>,
	pub thumbnails: Vec<ThumbnailState<'a>>,
	#[serde(rename = "durationSeconds")]
	pub duration_seconds: f32,
	pub id: &'a str,
	#[serde(rename = "isLive")]
	pub is_live: Option<bool>,
	#[serde(rename = "videoType")]
	pub video_type: Option<VideoType>,
	#[serde(rename = "metadataFilled")]
	pub metadata_filled: Option<bool>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailState<'a> {
	pub url: &'a str,
	pub width: u32,
	pub height: u32,
}