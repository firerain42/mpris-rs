//! This module contains the error handling code.
use std::fmt::Debug;


error_chain! {
    foreign_links {
        DBus(::dbus::Error);
    }
    errors {
        GeneralError(msg: String) {
            description("general error")
            display("general error: {}", msg)
        }
        AccessedAbsentOptionalProperty(obj_path: String, member: String) {
            description("accessed absent (optional) property")
            display("accessed absent optional property: '{}' '{}'", obj_path, member)
        }
        TypeBuildError(from: &'static str, to: String) {
            description("type build error")
            display("could not build type {} from '{}'", from, to)
        }
        TypeCastError(from: String, to: &'static str) {
            description("type cast error")
            display("could not cast type '{:?}' to {}", from, to)
        }
        ServiceUnknown(bus_name: String) {
            description("service unknown")
            display("The service {} is unknown. Is the player still running?", bus_name)
        }
    }
}

pub(crate) trait DebugStr {
    /// Returns the Debug string
    fn to_debug_str(&self) -> String;
}

impl<D> DebugStr for D where D: Debug {
    fn to_debug_str(&self) -> String {
        format!("{:?}", self)
    }
}

/// Returns `true` if `err`'s name contains `match_err_name`. If there is no `name`, `false` is
/// returned.
pub(crate) fn match_dbus_err(err: &::dbus::Error, match_err_name: &str) -> bool {
    err.name()
        .map(|name| name.contains(match_err_name))
        .unwrap_or(false)
}
