//! This module contains the error handling code.


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
            description("optional property is not present, but was accessed")
            display("accessed absent optional property: '{}' '{}'", obj_path, member)
        }
    }
}

/// Returns `true` if `err`'s name contains `match_err_name`. If there is no `name`, `false` is
/// returned.
pub(crate) fn match_dbus_err(err: &::dbus::Error, match_err_name: &str) -> bool {
    err.name()
        .map(|name| name.contains(match_err_name))
        .unwrap_or(false)
}
