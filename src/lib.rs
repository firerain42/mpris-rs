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

#![recursion_limit = "1024"]

extern crate dbus;
extern crate chrono;
#[macro_use]
extern crate error_chain;


pub mod client;
pub mod errors;


use dbus::{Path, MessageItem};
use dbus::arg::RefArg;
use chrono::{Utc, DateTime};
use std::collections::HashMap;
use std::str::FromStr;
use std::rc::Rc;
use errors::*;


/// A unique resource identifier.
type Uri = String;

/// A playback rate
///
/// This is a multiplier, so a value of 0.5 indicates that playback is happening at half speed,
/// while 1.5 means that 1.5 seconds of "track time" is consumed every second.
type PlaybackRate = f64;

/// Audio volume level
///
/// 0.0 means mute.
/// 1.0 is a sensible maximum volume level (ex: 0dB).
///
/// Note that the volume may be higher than 1.0, although generally clients should not attempt to
/// set it above 1.0.
type Volume = f64;

/// Time in microseconds.
type TimeInUs = f64;


/// Unique track identifier.
///
/// If the media player implements the TrackList interface and allows the same track to appear
/// multiple times in the tracklist, this must be unique within the scope of the tracklist.
///
/// Note that this should be a valid D-Bus object id, although clients should not assume that any
/// object is actually exported with any interfaces at that path.
///
/// Media players may not use any paths starting with `/org/mpris` unless explicitly allowed by this
/// specification. Such paths are intended to have special meaning, such as
/// `/org/mpris/MediaPlayer2/TrackList/NoTrack` to indicate "no track".
#[derive(Debug, Clone, PartialEq)]
pub struct TrackId {
    track_id: String,
}

impl FromStr for TrackId {
    type Err = Error;

    /// Creates new instance.
    fn from_str(track_id: &str) -> Result<Self> {
        if !Path::new(track_id).is_ok() {
            bail!(ErrorKind::TypeBuildError(stringify!(TrackId), track_id.to_string()))
        } else {
            Ok(TrackId { track_id: track_id.to_string() })
        }
    }
}

impl AsRef<str> for TrackId {
    fn as_ref(&self) -> &str {
        &self.track_id
    }
}

/// A playback state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackStatus {
    /// A track is currently playing.
    Playing,
    /// A track is currently paused.
    Paused,
    /// There is no track currently playing.
    Stopped,
}

impl FromStr for PlaybackStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<PlaybackStatus> {
        match s.to_lowercase().as_str() {
            "playing" => Ok(PlaybackStatus::Playing),
            "paused" => Ok(PlaybackStatus::Paused),
            "stopped" => Ok(PlaybackStatus::Stopped),
            _ => bail!(ErrorKind::TypeBuildError(stringify!(PlaybackStatus), s.to_string())),
                // 'forward-seek', 'reverse-seek' and 'error' are ignored
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
#[derive(Debug, Clone, PartialEq)]
pub enum LoopStatus {
    /// The playback will stop when there are no more tracks to play
    None,
    /// The current track will start again from the beginning once it has finished playing
    Track,
    /// The playback loops through a list of tracks
    Playlist,
}

impl FromStr for  LoopStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<LoopStatus> {
        match s.to_lowercase().as_str() {
            "none" => Ok(LoopStatus::None),
            "track" => Ok(LoopStatus::Track),
            "playlist" => Ok(LoopStatus::Playlist),
            _ => bail!(ErrorKind::TypeBuildError(stringify!(LoopStatus), s.to_string()))
        }
    }
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
pub struct MetadataMap {
    trackid: TrackId,
    raw_map: HashMap<String, Rc<RefArg>>,
}

impl MetadataMap {

    /// Creates a new `MetadataMap` from a Map of names and variants.
    pub fn from_map(raw_map: HashMap<String, Rc<RefArg>>) -> Result<Self> {
        let trackid;
        if let Some(track_id) = raw_map.get("mpris:trackid") {
            let track_id_str = track_id.as_str().chain_err(|| "Could not cast to str.")?;
            trackid = TrackId::from_str(track_id_str)?;
        } else {
            bail!("Mandatory 'mpris:trackid' is not present. Could not construct MetadataMap.");
        }

        Ok(MetadataMap { trackid , raw_map })
    }

    // MPRIS-specific
    /// A unique identity for this track within the context of an MPRIS object (eg: tracklist).
    pub fn trackid(&self) -> &TrackId { &self.trackid }
    /// The duration of the track in microseconds.
    pub fn length(&self) -> Option<TimeInUs> {
        use ::dbus::arg::cast;

        let argref = self.raw_map.get("mpris:length")?;
        Some(cast::<TimeInUs>(argref)?.clone())
    }
    /// The location of an image representing the track or album. Clients should not assume this
    /// will continue to exist when the media player stops giving out the URL.
    pub fn art_url(&self) -> Option<Uri> {
        unimplemented!();
    }

    // Common Xesam properties
    /// The album name.
    pub fn album(&self) -> Option<String> {
        unimplemented!();
    }
    /// The album artist(s).
    pub fn album_artist(&self) -> Option<Vec<String>> {
        unimplemented!();
    }
    /// The track artist(s).
    pub fn artist(&self) -> Option<Vec<String>> {
        unimplemented!();
    }
    /// The track lyrics.
    pub fn as_text(&self) -> Option<String> {
        unimplemented!();
    }
    /// The speed of the music, in beats per minute.
    pub fn audio_bpm(&self) -> Option<u32> {
        unimplemented!();
    }
    /// An automatically-generated rating, based on things such as how often it has been played.
    /// This should be in the range 0.0 to 1.0.
    pub fn auto_rating(&self) -> Option<f64> {
        unimplemented!();
    }
    /// A (list of) freeform comment(s).
    pub fn comment(&self) -> Option<Vec<String>> {
        unimplemented!();
    }
    /// The composer(s) of the track.
    pub fn composer(&self) -> Option<Vec<String>> {
        unimplemented!();
    }
    /// When the track was created. Usually only the year component will be useful.
    pub fn content_created(&self) -> Option<DateTime<Utc>> {
        unimplemented!();
    }
    /// The disc number on the album that this track is from.
    pub fn disc_number(&self) -> Option<u32> {
        unimplemented!();
    }
    /// When the track was first played.
    pub fn first_used(&self) -> Option<DateTime<Utc>> {
        unimplemented!();
    }
    /// The genre(s) of the track.
    pub fn genre(&self) -> Option<Vec<String>> {
        unimplemented!();
    }
    /// When the track was last played.
    pub fn last_used(&self) -> Option<DateTime<Utc>> {
        unimplemented!();
    }
    /// The lyricist(s) of the track.
    pub fn lyricist(&self) -> Option<Vec<String>> {
        unimplemented!();
    }
    /// The track title.
    pub fn title(&self) -> Option<String> {
        unimplemented!();
    }
    /// The location of the media file.
    pub fn track_number(&self) -> Option<u32> {
        unimplemented!();
    }
    /// The location of the media file.
    pub fn url(&self) -> Option<Uri> {
        unimplemented!();
    }
    /// The number of times the track has been played.
    pub fn use_count(&self) -> Option<u32> {
        unimplemented!();
    }
    /// A user-specified rating. This should be in the range 0.0 to 1.0.
    pub fn user_rating(&self) -> Option<f64> {
        unimplemented!();
    }
}

impl PartialEq for MetadataMap {
    fn eq(&self, other: &MetadataMap) -> bool {
        self.trackid == other.trackid
    }
}


#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::rc::Rc;
    use dbus::arg::RefArg;
    use std::str::FromStr;
    use super::*;

    fn test_MetadataMap() {
        let mut example_map: HashMap<String, Rc<RefArg>> = HashMap::with_capacity(22);
        example_map.insert("mpris:trackid".to_string(), Rc::new("/foo/bar/baz".to_string()));
        example_map.insert("mpris:length".to_string(), Rc::new(23 as super::TimeInUs));
        example_map.insert("mpris:artUrl".to_string(), Rc::new("/example/dir/art.png".to_string()));
        example_map.insert("xesam:album".to_string(), Rc::new("example album".to_string()));
        example_map.insert("xesam:albumArtist".to_string(), Rc::new(vec!["example album artist".to_string()]));
        example_map.insert("xesam:artist".to_string(), Rc::new(vec!["example artist".to_string()]));
        example_map.insert("xesam:asText".to_string(), Rc::new("example text".to_string()));
        example_map.insert("xesam:audioBPM".to_string(), Rc::new(23));
        example_map.insert("xesam:autoRating".to_string(), Rc::new(0.31415));
        example_map.insert("xesam:comment".to_string(), Rc::new(vec!["example comment".to_string()]));
        example_map.insert("xesam:composer".to_string(), Rc::new(vec!["example composer".to_string()]));
        example_map.insert("xesam:contentCreated".to_string(), Rc::new("2007-04-29T13:56+01:00".to_string()));
        example_map.insert("xesam:discNumber".to_string(), Rc::new(42));
        example_map.insert("xesam:firstUsed".to_string(), Rc::new("2008-04-29T13:56+01:00".to_string()));
        example_map.insert("xesam:genre".to_string(), Rc::new(vec!["example genre".to_string()]));
        example_map.insert("xesam:lastUsed".to_string(), Rc::new("2009-04-29T13:56+01:00".to_string()));
        example_map.insert("xesam:lyricist".to_string(), Rc::new(vec!["example lyricist".to_string()]));
        example_map.insert("xesam:title".to_string(), Rc::new("example title".to_string()));
        example_map.insert("xesam:trackNumber".to_string(), Rc::new(23));
        example_map.insert("xesam:url".to_string(), Rc::new("/example/dir/url.mp3".to_string()));
        example_map.insert("xesam:useCount".to_string(), Rc::new(42));
        example_map.insert("xesam:userRating".to_string(), Rc::new(0.31415));

        let mmap = MetadataMap::from_map(example_map).unwrap();
        assert_eq!(mmap.trackid(), &TrackId::from_str("/foo/bar/baz").unwrap());

        assert_eq!(mmap.length(), Some(23 as super::TimeInUs));
        assert_eq!(mmap.art_url(), Some("/example/dir/art.png".to_string()));
        assert_eq!(mmap.album(), Some("example album".to_string()));
        assert_eq!(mmap.album_artist(), Some(vec!["example album artist".to_string()]));
        assert_eq!(mmap.artist(), Some(vec!["example artist".to_string()]));
        assert_eq!(mmap.as_text(), Some("example text".to_string()));
        assert_eq!(mmap.audio_bpm(), Some(23));
        assert_eq!(mmap.auto_rating(), Some(0.31415));
        assert_eq!(mmap.comment(), Some(vec!["example comment".to_string()]));
        assert_eq!(mmap.composer(), Some(vec!["example composer".to_string()]));
        assert_eq!(mmap.content_created(), Some(FromStr::from_str("2007-04-29T13:56+01:00").unwrap()));
        assert_eq!(mmap.disc_number(), Some(42));
        assert_eq!(mmap.first_used(), Some(FromStr::from_str("2008-04-29T13:56+01:00").unwrap()));
        assert_eq!(mmap.genre(), Some(vec!["example genre".to_string()]));
        assert_eq!(mmap.last_used(), Some(FromStr::from_str("2009-04-29T13:56+01:00").unwrap()));
        assert_eq!(mmap.lyricist(), Some(vec!["example lyricist".to_string()]));
        assert_eq!(mmap.title(), Some("example title".to_string()));
        assert_eq!(mmap.track_number(), Some(23));
        assert_eq!(mmap.url(), Some("/example/dir/url.mp3".to_string()));
        assert_eq!(mmap.use_count(), Some(42));
        assert_eq!(mmap.user_rating(), Some(0.31415));
    }
}
