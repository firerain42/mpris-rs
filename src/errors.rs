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
    }
}
