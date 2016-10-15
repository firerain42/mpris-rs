//! This provides bindings for the
//! [MPRIS D-Bus interface](https://specifications.freedesktop.org/mpris-spec/2.2/).
//!
//! The Media Player Remote Interfacing Specification (MPRIS) is is a standard
//! [D-Bus](https://www.freedesktop.org/wiki/Software/dbus/) interface which aims to provide a
//! common programmatic API for controlling media players.
//!
//! It provides a mechanism for discovery, querying and basic playback control of compliant media
//! players, as well as a tracklist interface which is used to add context to the active media
//! item.

extern crate dbus;
extern crate time;

use std::vec::Vec;
use std::convert::From;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fmt;

use dbus::{Path, MessageItem};

pub type MprisResult<T> = Result<T, MprisError>;

/// A common error struct.
#[derive(Debug)]
pub struct MprisError {
    msg: String,
}

impl MprisError {
    fn new(msg: &str) -> Self {
        MprisError { msg: msg.to_string() }
    }
}

impl Display for MprisError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "MPRIS error: {}", self.msg)
    }
}

impl Error for MprisError {
    fn description(&self) -> &str {
        &self.msg
    }
}

impl From<String> for MprisError {
    fn from(msg: String) -> Self {
        MprisError::new(&msg)
    }
}

impl<'a> From<&'a str> for MprisError {
    fn from(msg: &'a str) -> Self {
        MprisError::new(msg)
    }
}

impl From<dbus::Error> for MprisError {
    fn from(err: dbus::Error) -> Self {
        let msg = match (err.message(), err.name()) {
            (Some(msg), Some(name)) => format!("D-Bus error: {} ({})", msg, name),
            (Some(msg), None) => format!("D-Bus error: {}", msg),
            (None, Some(name)) => format!("D-Bus error: ({})", name),
            (None, None) => format!("D-Bus error: unknown error"),
        };
        MprisError::new(&msg)
    }
}


/// A playback state.
#[derive(Debug, Clone, Copy)]
pub enum PlaybackStatus {
    /// A track is currently playing.
    Playing,
    /// A track is currently paused.
    Paused,
    /// There is no track currently playing.
    Stopped,
}

impl PlaybackStatus {
    pub fn from_str(s: &str) -> Option<PlaybackStatus> {
        match s.to_lowercase().as_str() {
            "playing" => Some(PlaybackStatus::Playing),
            "paused" => Some(PlaybackStatus::Paused),
            "stopped" => Some(PlaybackStatus::Stopped),
            _ => None,       // 'forward-seek', 'reverse-seek' and 'error' are ignored
        }
    }
}

impl Into<MessageItem> for PlaybackStatus {
    fn into(self) -> MessageItem {
        match self {
            PlaybackStatus::Playing => "Playing".into(),
            PlaybackStatus::Paused => "Paused".into(),
            PlaybackStatus::Stopped => "Stopped".into(),
        }
    }
}

/// A repeat / loop status
#[derive(Debug, Clone)]
pub enum LoopStatus {
    /// The playback will stop when there are no more tracks to play
    None,
    /// The current track will start again from the beginning once it has finished playing
    Track,
    /// The playback loops through a list of tracks
    Playlist,
}

impl Into<MessageItem> for LoopStatus {
    fn into(self) -> MessageItem {
        match self {
            LoopStatus::None => "None".into(),
            LoopStatus::Track => "Track".into(),
            LoopStatus::Playlist => "Playlist".into(),
        }
    }
}

/// The metadata of a track
#[derive(Debug, Clone)]
pub struct Metadata<'a> {
    // MPRIS-specific
    /// A unique identity for this track within the context of an MPRIS object (eg: tracklist).
    pub trackid: Path<'a>,
    /// The duration of the track in microseconds.
    pub length: Option<u64>,
    /// The location of an image representing the track or album. Clients should not assume this
    /// will continue to exist when the media player stops giving out the URL.
    pub art_url: Option<String>,

    // Common Xesam properties
    /// The album name.
    pub album: Option<String>,
    /// The album name.
    pub album_artist: Option<Vec<String>>,
    /// The album artist(s).
    pub artist: Option<Vec<String>>,
    /// The track artist(s).
    pub as_text: Option<String>,
    /// The track lyrics.
    pub audio_bpm: Option<u32>,
    /// The speed of the music, in beats per minute.
    pub auto_rating: Option<f64>,
    /// An automatically-generated rating, based on things such as how often it has been played.
    /// This should be in the range 0.0 to 1.0.
    pub comment: Option<Vec<String>>,
    /// A (list of) freeform comment(s).
    pub composer: Option<Vec<String>>,
    /// The composer(s) of the track.
    pub content_created: Option<time::Tm>,
    /// When the track was created. Usually only the year component will be useful.
    pub disc_number: Option<u32>,
    /// The disc number on the album that this track is from.
    pub first_used: Option<time::Tm>,
    /// When the track was first played.
    pub genre: Option<Vec<String>>,
    /// The genre(s) of the track.
    pub last_used: Option<time::Tm>,
    /// When the track was last played.
    pub lyricist: Option<Vec<String>>,
    /// The lyricist(s) of the track.
    pub title: Option<String>,
    /// The track title.
    pub track_number: Option<u32>,
    /// The location of the media file.
    pub url: Option<String>,
    /// The number of times the track has been played.
    pub use_count: Option<u32>,
    /// A user-specified rating. This should be in the range 0.0 to 1.0.
    pub user_rating: Option<f64>,
}

impl<'a> Metadata<'a> {
    pub fn new(trackid: Path<'a>) -> Self {
        Metadata {
            trackid: trackid,
            length: None,
            art_url: None,

            album: None,
            album_artist: None,
            artist: None,
            as_text: None,
            audio_bpm: None,
            auto_rating: None,
            comment: None,
            composer: None,
            content_created: None,
            disc_number: None,
            first_used: None,
            genre: None,
            last_used: None,
            lyricist: None,
            title: None,
            track_number: None,
            url: None,
            use_count: None,
            user_rating: None,
        }
    }
}

/// (Maybe) appends an `MessageItem` with its name to a dictionary
fn add_item<M: Into<MessageItem>>(items: &mut Vec<Result<(String, MessageItem), ()>>,
                                  name: &str,
                                  value: Option<M>) {
    if let Some(i) = value {
        items.push(Ok((name.to_string(), i.into())));
    }
}

/// Converts a datetime to a `MessageItem`
fn dt2mi(dt: time::Tm) -> MessageItem {
    format!("{}", dt.rfc3339()).into()
}

/// Converts a vector to a `MessageItem`
fn vec2mi(vec: Vec<String>) -> MessageItem {
    (&vec as &[String]).into()
}

impl<'a> From<Metadata<'a>> for MessageItem {
    fn from(md: Metadata<'a>) -> MessageItem {
        let mut items: Vec<Result<(String, MessageItem), _>> = Vec::new();

        let static_trackid: Path<'static> = (&md.trackid).to_string().into();
        items.push(Ok(("trackid".to_string(), static_trackid.into())));

        add_item(&mut items, "length", md.length);
        add_item(&mut items, "art_url", md.art_url);
        add_item(&mut items, "album", md.album);
        add_item(&mut items, "album_artist", md.album_artist.map(vec2mi));
        add_item(&mut items, "artist", md.artist.map(vec2mi));
        add_item(&mut items, "as_text", md.as_text);
        add_item(&mut items, "audio_bpm", md.audio_bpm);
        add_item(&mut items, "auto_rating", md.auto_rating);
        add_item(&mut items, "comment", md.comment.map(vec2mi));
        add_item(&mut items, "composer", md.composer.map(vec2mi));
        add_item(&mut items, "content_created", md.content_created.map(dt2mi));
        add_item(&mut items, "disc_number", md.disc_number);
        add_item(&mut items, "first_used", md.first_used.map(dt2mi));
        add_item(&mut items, "genre", md.genre.map(vec2mi));
        add_item(&mut items, "last_used", md.last_used.map(dt2mi));
        add_item(&mut items, "lyricist", md.lyricist.map(vec2mi));
        add_item(&mut items, "title", md.title);
        add_item(&mut items, "track_number", md.track_number);
        add_item(&mut items, "url", md.url);
        add_item(&mut items, "use_count", md.use_count);
        add_item(&mut items, "user_rating", md.user_rating);

        MessageItem::from_dict(items.into_iter()).unwrap()
    }
}
