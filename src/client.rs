use dbus::{BusType, Connection, Message, Props, PropHandler, MessageItem};
use std::rc::Rc;
use std::collections::BTreeMap;

use errors::*;

struct DBusConn {
    conn: Connection,
    bus_name: String,
    timeout: i32,
}

impl DBusConn {
    fn call_method_without_reply(&self, obj_path: &str, member: &str) -> Result<()> {
        let msg =
            Message::new_method_call(&self.bus_name, obj_path, "org.mpris.MediaPlayer2", member)?;

        self.conn
            .send_with_reply_and_block(msg, self.timeout)
            .chain_err(|| ErrorKind::GeneralError("Could not call D-Bus method.".to_string()))?;
        Ok(())
    }

    fn get_prop(&self, obj_path: &str, member: &str) -> Result<MessageItem> {
        let prop = Props::new(&self.conn,
                              &self.bus_name,
                              obj_path,
                              "org.mpris.MediaPlayer2",
                              self.timeout);
        let msg_item = prop.get(member)?;
        Ok(msg_item)
    }

    fn get_optional_prop(&self, obj_path: &str, member: &str) -> Result<Option<MessageItem>> {
        let prop = Props::new(&self.conn,
                              &self.bus_name,
                              obj_path,
                              "org.mpris.MediaPlayer2",
                              self.timeout);
        match prop.get(member) {
            Ok(msg_item) => Ok(Some(msg_item)),
            Err(e) => {
                if e.message()
                    .map(|msg| msg.contains("was not found"))
                    .unwrap_or(false) {
                    Ok(None)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    fn set_prop(&self, obj_path: &str, member: &str, value: MessageItem) -> Result<()> {
        let prop = Props::new(&self.conn,
                              &self.bus_name,
                              obj_path,
                              "org.mpris.MediaPlayer2",
                              self.timeout);
        prop.set(member, value)?;
        Ok(())
    }


    /// Constructs a new `DBusConn` for `org.mpris.MediaPlayer2.playerName`.
    ///
    /// `timeout_ms` specifies the maximum time a D-Bus method call blocks. The value -1 disables
    /// the timeout.
    fn new(player_name: &str, timeout_ms: i32) -> Result<Self> {
        let conn = Connection::get_private(BusType::Session)?;
        Ok(DBusConn {
            conn,
            bus_name: format!("org.mpris.MediaPlayer2.{}", player_name),
            timeout: timeout_ms,
        })
    }
}


pub struct MprisClient {
    dbus_conn: Rc<DBusConn>,

    pub root: MprisRoot,
}

impl MprisClient {
    pub fn new(player_name: &str, timeout_ms: i32) -> Result<Self> {
        let dbus_conn = Rc::new(DBusConn::new(player_name, timeout_ms)?);

        let dbus_conn_clone = dbus_conn.clone();

        Ok(MprisClient {
            dbus_conn,

            root: MprisRoot::new(dbus_conn_clone),
        })
    }
}


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
        self.dbus_conn.call_method_without_reply("/org/mpris/MediaPlayer2", "Raise")
    }

    /// Causes the media player to stop running.
    ///
    /// The media player may refuse to allow clients to shut it down. In this case, the `can_quit`
    /// property is `false` and this method does nothing.
    pub fn quit(&self) -> Result<()> {
        self.dbus_conn.call_method_without_reply("/org/mpris/MediaPlayer2", "Quit")
    }

    /// If `false`, calling `quit` will have no effect, and may raise an error. If `true`,
    /// calling `quit` will cause the media application to attempt to quit (although it may still be
    /// prevented from quitting by the user, for example).
    ///
    /// When this property changes, the `org.freedesktop.DBus.Properties.PropertiesChanged` signal
    /// is emitted with the new value.
    pub fn can_quit(&self) -> Result<bool> {
        match self.dbus_conn.get_prop("/org/mpris/MediaPlayer2", "CanQuit") {
            Ok(MessageItem::Bool(cq)) => Ok(cq),
            Ok(_) => {
                Err(ErrorKind::GeneralError("Could not get property: unexpected type".to_string())
                    .into())
            }
            Err(err) => Err(err.into()),
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
        match self.dbus_conn.get_optional_prop("/org/mpris/MediaPlayer2", "Fullscreen") {
            Ok(Some(MessageItem::Bool(cq))) => Ok(Some(cq)),
            Ok(None) => Ok(None),
            Err(err) => Err(err.into()),
            Ok(_) => {
                Err(ErrorKind::GeneralError("Could not get property: unexpected type".to_string())
                    .into())
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
    /// If `can_set_fullscreen` is `false`, then attempting to set this property should have no effect,
    /// and may raise an error. However, even if it is `true`, the media player may still be unable
    /// to fulfil the request, in which case attempting to set this property will have no effect
    /// (but should not raise an error).
    ///
    /// When this property changes, the `org.freedesktop.DBus.Properties.PropertiesChanged` signal
    /// is emitted with the new value.
    ///
    /// This property is optional.
    pub fn set_fullscreen(&self, value: bool) -> Result<()> {
        self.dbus_conn.set_prop("/org/mpris/MediaPlayer2",
                                "Fullscreen",
                                MessageItem::Bool(value))
    }
}
