extern crate mpris;

use mpris::errors;


use mpris::client::MprisClient;

fn setup_vlc() -> MprisClient {
    MprisClient::new("vlc", 1000).expect(
        "Could not connect to VLC's MPRIS Interface. Is VLC running?",
    )
}

fn setup_cantata() -> MprisClient {
    MprisClient::new("cantata", 1000).expect(
        "Could not connect to Cantata's MPRIS Interface. Is Cantata running?",
    )
}

#[test]
fn test_raise() {
    let client = setup_vlc();
    client.root.raise().unwrap();
}

#[test]
fn test_can_quit() {
    let client = setup_vlc();
    assert!(client.root.can_quit().unwrap());
}

#[test]
fn test_fullscreen() {
    let client = setup_vlc();
    let is_fullscreen = client.root.fullscreen().unwrap().unwrap();
    client.root.set_fullscreen(!is_fullscreen).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    let is_fullscreen2 = client.root.fullscreen().unwrap().unwrap();

    assert!(!is_fullscreen, is_fullscreen2);
}

#[test]
fn test_fullscreen_optional() {
    let client = setup_cantata();
    let result = client.root.fullscreen();
    let is_fullscreen = result.unwrap();
    assert_eq!(None, is_fullscreen);

    let method_call_res = client.root.set_fullscreen(false);
    match method_call_res {
        Err(errors::Error(errors::ErrorKind::AccessedAbsentOptionalProperty(..), ..)) => {}
        Err(e) => panic!("wrong kind of error: {:?}", e),
        Ok(..) => panic!("error expected"),
    }
}
