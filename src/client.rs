use dbus::{BusType, Connection, Message, Props, MessageItem};

use super::{MprisError, MprisResult};

macro_rules! try_mpris {
    ($e:expr) => (try!($e.map_err(MprisError::from)));
    ($e:expr, $msg:expr) => (try!($e.map_err(|_| MprisError::new($msg))));
}

pub struct MprisClient {
    conn: Connection,
    bus_name: String,
    timeout: i32,
}

impl MprisClient {
    fn call_method_without_reply(&self, obj_path: &str, member: &str) -> MprisResult<()> {
        let msg = try_mpris!(Message::new_method_call(&self.bus_name,
                                                      obj_path,
                                                      "org.mpris.MediaPlayer2",
                                                      member));

        try_mpris!(self.conn.send_with_reply_and_block(msg, self.timeout),
                   "Could not call D-Bus method.");
        Ok(())
    }

    fn get_prop(&self, obj_path: &str, member: &str) -> MprisResult<MessageItem> {
        let prop = Props::new(&self.conn,
                              &self.bus_name,
                              obj_path,
                              "org.mpris.MediaPlayer2",
                              self.timeout);
        let msg_item = try_mpris!(prop.get(member));
        Ok(msg_item)
    }

    fn set_prop(&self, obj_path: &str, member: &str, value: MessageItem) -> MprisResult<()> {
        let prop = Props::new(&self.conn,
                              &self.bus_name,
                              obj_path,
                              "org.mpris.MediaPlayer2",
                              self.timeout);
        try_mpris!(prop.set(member, value));
        Ok(())
    }


    /// Constructs a new `MprisClient` for `org.mpris.MediaPlayer2.playerName`.
    ///
    /// `timeout_ms` specifies the maximum time a D-Bus method call blocks. The vagetlue -1 disables
    /// the timeout.
    pub fn new(player_name: &str, timeout_ms: i32) -> MprisResult<Self> {
        let conn = try_mpris!(Connection::get_private(BusType::Session));
        Ok(MprisClient {
            conn: conn,
            bus_name: format!("org.mpris.MediaPlayer2.{}", player_name),
            timeout: timeout_ms,
        })
    }

    /// Brings the media player's user interface to the front using any appropriate mechanism
    /// available.
    pub fn raise(&self) -> Result<(), MprisError> {
        self.call_method_without_reply("/org/mpris/MediaPlayer2", "Raise")
    }

    /// Causes the media player to stop running.
    ///
    /// The media player may refuse to allow clients to shut it down. In this case, the `can_quit`
    /// property is `false` and this method does nothing.
    pub fn quit(&self) -> Result<(), MprisError> {
        self.call_method_without_reply("/org/mpris/MediaPlayer2", "Quit")
    }

    /// If `false`, calling `quit` will have no effect, and may raise an error. If `true`,
    /// calling `quit` will cause the media application to attempt to quit (although it may still be
    /// prevented from quitting by the user, for example).
    ///
    /// When this property changes, the `org.freedesktop.DBus.Properties.PropertiesChanged` signal
    /// is emitted with the new value.
    pub fn can_quit(&self) -> MprisResult<bool> {
        match self.get_prop("/org/mpris/MediaPlayer2", "CanQuit") {
            Ok(MessageItem::Bool(cq)) => Ok(cq),
            Err(err) => Err(MprisError::from(err)),
            Ok(_) => Err(MprisError::new("Could not get property.")),
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
    pub fn fullscreen(&self) -> MprisResult<bool> {
        match self.get_prop("/org/mpris/MediaPlayer2", "Fullscreen") {
            Ok(MessageItem::Bool(cq)) => Ok(cq),
            Err(err) => Err(MprisError::from(err)),
            Ok(_) => Err(MprisError::new("Could not get property.")),
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
    /// If `can_set_fullscreen` is `false`, then attempting to set this property should have no effect,
    /// and may raise an error. However, even if it is `true`, the media player may still be unable
    /// to fulfil the request, in which case attempting to set this property will have no effect
    /// (but should not raise an error).
    ///
    /// When this property changes, the `org.freedesktop.DBus.Properties.PropertiesChanged` signal
    /// is emitted with the new value.
    ///
    /// This property is optional.
    pub fn set_fullscreen(&self, value: bool) -> MprisResult<()> {
        self.set_prop("/org/mpris/MediaPlayer2", "Fullscreen", MessageItem::Bool(value))
    }
}
