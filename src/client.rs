use dbus::{BusType, Connection, Message, Props, MessageItem, MessageType};
use dbus::arg::{RefArg, Variant};
use std::rc::Rc;
use std::str::FromStr;
use std::collections::HashMap;

use errors::*;

/// Abstraction over the DBUS connection. All interactions with DBUS should go through one of the
/// methods of this struct.
#[derive(Debug)]
struct DBusConn {
    conn: Connection,
    bus_name: String,
    unique_bus_name: String,
    timeout: i32,
}

impl DBusConn {
    /// Calls a DBUS method without returning a value. This method blocks until the call either
    /// succeeds or fails.
    fn call_method_without_reply(&self, obj_path: &str, member: &str) -> Result<()> {
        let msg =
            Message::new_method_call(&self.bus_name, obj_path, "org.mpris.MediaPlayer2", member)?;
        if let Err(err) = self.conn.send_with_reply_and_block(msg, self.timeout) {
            if err.message().unwrap_or("").contains("org.freedesktop.DBus.Error.ServiceUnknown") {
                Err(err).chain_err(|| ErrorKind::ServiceUnknown(self.bus_name.clone()))
            } else {
                Err(err).chain_err(|| ErrorKind::GeneralError("Could not call D-Bus method.".to_string()))
            }
        } else { Ok(())}
    }

    /// Reads a DBUS property.
    fn get_prop(&self, obj_path: &str, member: &str) -> Result<MessageItem> {
        let prop = Props::new(
            &self.conn,
            &self.bus_name,
            obj_path,
            "org.mpris.MediaPlayer2",
            self.timeout,
        );
        let msg_item = prop.get(member)?;
        Ok(msg_item)
    }

    /// Safely reads an optional DBUS property.
    fn get_optional_prop(&self, obj_path: &str, member: &str) -> Result<Option<MessageItem>> {
        let prop = Props::new(
            &self.conn,
            &self.bus_name,
            obj_path,
            "org.mpris.MediaPlayer2",
            self.timeout,
        );
        match prop.get(member) {
            Ok(msg_item) => Ok(Some(msg_item)),
            Err(ref e) if match_dbus_err(e, "DBus.Error.UnknownProperty") => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Writes a DBUS property.
    fn set_prop(&self, obj_path: &str, member: &str, value: MessageItem) -> Result<()> {
        let prop = Props::new(
            &self.conn,
            &self.bus_name,
            obj_path,
            "org.mpris.MediaPlayer2",
            self.timeout,
        );
        match prop.set(member, value) {
            Ok(..) => Ok(()),
            Err(ref e) if match_dbus_err(e, "DBus.Error.UnknownProperty") => {
                bail!(ErrorKind::AccessedAbsentOptionalProperty(
                    obj_path.to_string(),
                    member.to_string(),
                ))
            }
            Err(e) => bail!(e),
        }
    }


    /// Constructs a new `DBusConn` for `org.mpris.MediaPlayer2.playerName`.
    ///
    /// `timeout_ms` specifies the maximum time a D-Bus method call blocks. The value -1 disables
    /// the timeout.
    fn new(player_name: &str, timeout_ms: i32) -> Result<Self> {
        let conn = Connection::get_private(BusType::Session)?;

        conn.add_match(
            "path='/org/mpris/MediaPlayer2',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged'",
        )?;
        conn.add_match(
            "path='/org/mpris/MediaPlayer2',interface='org.mpris.MediaPlayer2'",
        )?;
        conn.add_match(
            "path='/org/mpris/MediaPlayer2',interface='org.mpris.MediaPlayer2.Player'",
        )?;
        conn.add_match(
            "path='/org/mpris/MediaPlayer2',interface='org.mpris.MediaPlayer2.TrackList'",
        )?;
        conn.add_match(
            "path='/org/mpris/MediaPlayer2',interface='org.mpris.MediaPlayer2.Playlists'",
        )?;

        let bus_name = format!("org.mpris.MediaPlayer2.{}", player_name);

        // get unique bus name
        let msg = Message::new_method_call("org.freedesktop.DBus",
                                           "/org/freedesktop/DBus",
                                           "org.freedesktop.DBus", "GetNameOwner")
            .expect("Could not construct method call.")
            .append1(&bus_name);
        let res = conn.send_with_reply_and_block(msg, timeout_ms)
            .chain_err(|| ErrorKind::GeneralError("Could not get unique bus name. Does the player exist?".to_string()))?;
        let unique_name = res.read1().chain_err(|| "Could not convert to String")?;

        Ok(DBusConn {
            conn,
            bus_name,
            unique_bus_name: unique_name,
            timeout: timeout_ms,
        })
    }
}

#[derive(Debug)]
pub struct MprisClient {
    dbus_conn: Rc<DBusConn>,

    pub root: MprisRoot,
}

impl MprisClient {
    /// Creates a new `MprisClient` instance.
    ///
    /// `timeout_ms` specifies the maximum time a D-Bus method call blocks. The value -1 disables
    /// the timeout.
    pub fn new(player_name: &str, timeout_ms: i32) -> Result<Self> {
        let dbus_conn = Rc::new(DBusConn::new(player_name, timeout_ms)?);

        let dbus_conn_clone = dbus_conn.clone();

        Ok(MprisClient {
            dbus_conn,

            root: MprisRoot::new(dbus_conn_clone),
        })
    }

    /// Lists all available media players.
    ///
    /// `timeout_ms` specifies the maximum time a D-Bus method call blocks. The value -1 disables
    /// the timeout.
    pub fn list_players(timeout_ms: i32) -> Result<Vec<String>> {
        let conn = Connection::get_private(BusType::Session)?;
        let msg = Message::new_method_call("org.freedesktop.DBus",
                                           "/org/freedesktop/DBus",
                                           "org.freedesktop.DBus",
                                           "ListNames")
            .expect("Could not construct method call.");
        let reply = conn.send_with_reply_and_block(msg, timeout_ms)?;
        let buses: Vec<String> = reply.read1().chain_err(|| "Could not typecast return value")?;
        Ok(buses.into_iter()
            .filter(|bus| { bus.starts_with("org.mpris.MediaPlayer2.") })
            .map(|bus| { bus[23..].to_string() })
            .collect())
    }

    /// Returns an iterator of `MprisSignal`s.`timeout_ms` specifies the maximum amount of time the
    /// iterator blocks (and waits for new messages).
    pub fn signals(&self, timeout_ms: u32) -> MprisSignals {
        MprisSignals::new(self.dbus_conn.clone(), timeout_ms)
    }
}

#[derive(Debug)]
pub struct MprisRoot {
    dbus_conn: Rc<DBusConn>,
}


impl MprisRoot {
    fn new(dbus_conn: Rc<DBusConn>) -> Self {
        MprisRoot { dbus_conn }
    }

    /// Brings the media player's user interface to the front using any appropriate mechanism
    /// available.
    pub fn raise(&self) -> Result<()> {
        self.dbus_conn.call_method_without_reply(
            "/org/mpris/MediaPlayer2",
            "Raise",
        )
    }

    /// Causes the media player to stop running.
    ///
    /// The media player may refuse to allow clients to shut it down. In this case, the `can_quit`
    /// property is `false` and this method does nothing.
    pub fn quit(&self) -> Result<()> {
        self.dbus_conn.call_method_without_reply(
            "/org/mpris/MediaPlayer2",
            "Quit",
        )
    }

    /// If `false`, calling `quit` will have no effect, and may raise an error. If `true`,
    /// calling `quit` will cause the media application to attempt to quit (although it may still be
    /// prevented from quitting by the user, for example).
    ///
    /// When this property changes, the `org.freedesktop.DBus.Properties.PropertiesChanged` signal
    /// is emitted with the new value.
    pub fn can_quit(&self) -> Result<bool> {
        match self.dbus_conn.get_prop(
            "/org/mpris/MediaPlayer2",
            "CanQuit",
        ) {
            Ok(MessageItem::Bool(cq)) => Ok(cq),
            Ok(_) => {
                Err(
                    ErrorKind::GeneralError("Could not get property: unexpected type".to_string())
                        .into(),
                )
            }
            Err(err) => Err(err),
        }
    }

    /// Whether the media player is occupying the fullscreen.
    ///
    /// This is typically used for videos. A value of `true` indicates that the media player is
    /// taking up the full screen.
    ///
    /// Media centre software may well have this value fixed to `true`
    ///
    /// When this property changes, the `org.freedesktop.DBus.Properties.PropertiesChanged` signal
    /// is emitted with the new value.
    ///
    /// This property is optional.
    pub fn fullscreen(&self) -> Result<Option<bool>> {
        match self.dbus_conn.get_optional_prop(
            "/org/mpris/MediaPlayer2",
            "Fullscreen",
        ) {
            Ok(Some(MessageItem::Bool(cq))) => Ok(Some(cq)),
            Ok(None) => Ok(None),
            Err(err) => Err(err),
            Ok(_) => {
                Err(
                    ErrorKind::GeneralError("Could not get property: unexpected type".to_string())
                        .into(),
                )
            }
        }
    }

    /// Whether the media player is occupying the fullscreen.
    ///
    /// This is typically used for videos. A value of `true` indicates that the media player is
    /// taking up the full screen.
    ///
    /// If `can_set_fullscreen` is `true`, clients may set this property to `true` to tell the media
    /// player to enter fullscreen mode, or to `false` to return to windowed mode.
    ///
    /// If `can_set_fullscreen` is `false`, then attempting to set this property should have no
    /// effect, and may raise an error. However, even if it is `true`, the media player may still be
    /// unable to fulfil the request, in which case attempting to set this property will have no
    /// effect (but should not raise an error).
    ///
    /// When this property changes, the `org.freedesktop.DBus.Properties.PropertiesChanged` signal
    /// is emitted with the new value.
    ///
    /// This property is optional.
    pub fn set_fullscreen(&self, value: bool) -> Result<()> {
        self.dbus_conn.set_prop(
            "/org/mpris/MediaPlayer2",
            "Fullscreen",
            MessageItem::Bool(value),
        )
    }
}

/// Iterator over `MprisSignal`s.
pub struct MprisSignals {
    dbus_conn: Rc<DBusConn>,
    timeout_ms: u32,
}

impl MprisSignals {
    /// Creates new `MprisSignals` instance.
    fn new(dbus_conn: Rc<DBusConn>, timeout_ms: u32) -> Self {
        MprisSignals { dbus_conn, timeout_ms }
    }
}

impl Iterator for MprisSignals {
    type Item = MprisSignal;

    fn next(&mut self) -> Option<Self::Item> {
        self
            .dbus_conn
            .conn
            .incoming(self.timeout_ms)
            .filter(|msg| {
                if let Some(msg_str) = msg.sender() {
                    &msg_str as &str == self.dbus_conn.unique_bus_name
                        || &msg_str as &str == self.dbus_conn.bus_name
                } else { false }
            })
            .filter_map(|msg| MprisSignal::from_message(&msg))
            .next()
    }
}

/// Enum for the signals emitted by an MPRIS interface.
#[derive(PartialEq, Debug, Clone)]
pub enum MprisSignal {
    /// Indicates that the track position has changed in a way that is inconsistant with the current
    /// playing state.
    ///
    /// When this signal is not received, clients should assume that:
    /// - When playing, the position progresses according to the rate property.
    /// - When paused, it remains constant.
    ///
    /// This signal does not need to be emitted when playback starts or when the track changes,
    /// unless the track is starting at an unexpected position. An expected position would be the
    /// last known one when going from `Paused` to `Playing`, and 0 when going from `Stopped` to
    /// `Playing`.
    Seeked { position: i64 },
    // todo MPRIS TrackList
//    /// Indicates that the entire tracklist has been replaced.
//    /// It is left up to the implementation to decide when a change to the track list is invasive
//    /// enough that this signal should be emitted instead of a series of `TrackAdded` and
//    /// `TrackRemoved` signals.
//    TrackListReplaced { tracks: Vec<::TrackId>, current_track: ::TrackId },
//    /// Indicates that a track has been added to the track list.
//    TrackAdded { metadata: ::MetadataMap, after_track: ::TrackId },
//    /// Indicates that a track has been removed from the track list.
//    TrackRemoved { track_id: ::TrackId },
//    /// Indicates that the metadata of a track in the tracklist has changed.
//    /// This may indicate that a track has been replaced, in which case the `track_id` metadata
//    /// entry is different from the `track_id` argument.
//    TrackMetadataChanged { track_id: ::TrackId, metadata: ::MetadataMap },
    // todo MPRIS Playlists
//    /// Indicates that either the Name or Icon attribute of a playlist has changed.
//    /// Client implementations should be aware that this signal may not be implemented.
//    PlaylistChanged { playlist: ::Playlist },
    /// Indicates that a properties have changed or have been invalidated.
    PropertiesChanged {
        interface: String,
        changed_properties: Vec<ChangedProperty>,
        invalidated_properties: Vec<String>,
    },
}

impl MprisSignal {
    /// Builds a new `MprisSignal` from a DBUS `Message`.
    ///
    /// Only signals with the sender bus name "org.freedesktop.DBus" and `bus_name` are considered.
    fn from_message(msg: &Message) -> Option<Self> {
        if let (MessageType::Signal, Some(_path), Some(_interface), Some(_member)) = msg.headers() {
            match (&_path as &str, &_interface as &str, &_member as &str) {
                ("/org/mpris/MediaPlayer2", "org.freedesktop.DBus.Properties", "PropertiesChanged") => {
                    if let (Some(interface),
                        Some(ch_props),
                        Some(invalidated_properties)) =
                    msg.get3::<_, HashMap<String, Variant<Box<RefArg>>>, _>() {
                        let changed_properties = ch_props
                            .into_iter()
                            .filter_map(|(n, mut v)| ChangedProperty::from_variant(&n, &mut v).ok())
                            .collect();
                        Some(MprisSignal::PropertiesChanged {
                            interface,
                            changed_properties,
                            invalidated_properties,
                        })
                    } else { None }
                }
                ("/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player", "Seeked") => {
                    if let Some(pos) = msg.get1::<i64>() {
                        Some(MprisSignal::Seeked { position: pos })
                    } else { None }
                }
                // todo MPRIS TrackList
                // todo MPRIS Playlists
                _ => None
            }
        } else { None }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ChangedProperty {
    // Mpris root properties
    CanQuit(bool),
    Fullscreen(bool),
    CanSetFullscreen(bool),
    CanRaise(bool),
    HasTrackList(bool),
    Identity(String),
    DesktopEntry(String),
    SupportedUriSchemes(Vec<String>),
    SupportedMimeTypes(Vec<String>),

    // Mpris Player properties
    PlaybackStatus(::PlaybackStatus),
    LoopStatus(::LoopStatus),
    Rate(::PlaybackRate),
    Shuffle(bool),
    Metadata(::MetadataMap),
    Volume(::Volume),
    MinimumRate(::PlaybackRate),
    MaximumRate(::PlaybackRate),
    CanGoNext(bool),
    CanGoPrevious(bool),
    CanPlay(bool),
    CanPause(bool),
    CanSeek(bool),

    // Mpris TrackList properties
    Tracks,
    CanEditTracks(bool),

    // todo MPRIS Playlists
// Mpris Playlists properties
//    PlaylistCount(u32),
//    Orderings(::PlaylistOrdering),
//    ActivePlaylist(::Playlist),

    Other(String),
}

impl ChangedProperty {
    fn from_variant(name: &str, data: &mut Variant<Box<RefArg>>) -> Result<Self> {
        use client::ChangedProperty::*;

        let res = match name {
            // Mpris root properties
            "CanQuit" => CanQuit(cast_var(data)?),
            "Fullscreen" => Fullscreen(cast_var(data)?),
            "CanSetFullscreen" => CanSetFullscreen(cast_var(data)?),
            "CanRaise" => CanRaise(cast_var(data)?),
            "HasTrackList" => HasTrackList(cast_var(data)?),
            "Identity" => Identity(cast_var_to_str(data)?.to_string()),
            "DesktopEntry" => DesktopEntry(cast_var_to_str(data)?.to_string()),
            "SupportedUriSchemes" => SupportedUriSchemes(cast_var::<Vec<String>>(data)?.clone()),
            "SupportedMimeTypes" => SupportedMimeTypes(cast_var::<Vec<String>>(data)?.clone()),

// Mpris Player properties
            "PlaybackStatus" => PlaybackStatus(::PlaybackStatus::from_str(cast_var_to_str(data)?)?),
            "LoopStatus" => LoopStatus(::LoopStatus::from_str(cast_var_to_str(data)?)?),
            "Rate" => Rate(cast_var(data)?),
            "Shuffle" => Shuffle(cast_var(data)?),
            "Metadata" => {
                if let Some(raw_map_variant) = ::dbus::arg::cast_mut::<HashMap<String, Variant<Box<RefArg>>>>(&mut data.0) {
                    let raw_map: HashMap<String, Rc<RefArg>> = raw_map_variant.into_iter()
                        .map(|(k, v)| {
                            let value_data: Box<RefArg> = ::std::mem::replace(&mut v.0, Box::new(false));
                            (k.to_string(), value_data.into())
                        })
                        .collect();

                    return Ok(Metadata(::MetadataMap::from_map(raw_map)?));
                }
                bail!(ErrorKind::TypeCastError(data.to_debug_str(), "HashMap"));
            }
            "Volume" => Volume(cast_var(data)?),
            "MinimumRate" => MinimumRate(cast_var(data)?),
            "MaximumRate" => MaximumRate(cast_var(data)?),
            "CanGoNext" => CanGoNext(cast_var(data)?),
            "CanGoPrevious" => CanGoPrevious(cast_var(data)?),
            "CanPlay" => CanPlay(cast_var(data)?),
            "CanPause" => CanPause(cast_var(data)?),
            "CanSeek" => CanSeek(cast_var(data)?),

// Mpris TrackList properties
            "Tracks" => Tracks,
            "CanEditTracks" => CanEditTracks(cast_var(data)?),

// Mpris Playlists properties
// "PlaylistCount" => PlaylistCount(*data.as_any().downcast_ref::<u32>()?),
// "Orderings" => Orderings(*data.as_any().downcast_ref::<Vec<String>>()?),
// "ActivePlaylist" => ActivePlaylist( ... ),
            _ => Other(format!("{:?}", data.0)),
        };

        Ok(res)
    }
}


fn cast_var_to_str(var: &Variant<Box<RefArg>>) -> Result<&str> {
    var.0.as_str().ok_or_else(|| ErrorKind::TypeCastError(var.to_debug_str(), "&str").into())
}

fn cast_var<T: Clone + 'static>(var: &Variant<Box<RefArg>>) -> Result<T> {
    ::dbus::arg::cast::<T>(&var.0)
        .cloned()
        .ok_or_else(|| ErrorKind::TypeCastError(var.to_debug_str(), stringify!(T)).into())
}

