use super::*;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

const KEY_G: u16 = 34;
const KEY_LEFTSHIFT: u16 = 42;
const OPEN_ATTEMPTS: u32 = 50;
const OPEN_RETRY: Duration = Duration::from_millis(100);
const READ_TIMEOUT: Duration = Duration::from_secs(10);

fn event_node(device: &mut UinputDevice) -> PathBuf {
    device
        .0
        .enumerate_dev_nodes_blocking()
        .unwrap()
        .filter_map(Result::ok)
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("event"))
        })
        .unwrap()
}

fn open_reader(node: &PathBuf) -> evdev::Device {
    for _ in 0..OPEN_ATTEMPTS {
        if let Ok(reader) = evdev::Device::open(node) {
            return reader;
        }
        std::thread::sleep(OPEN_RETRY);
    }
    evdev::Device::open(node).unwrap()
}

fn read_key_events(mut reader: evdev::Device, count: usize) -> Vec<(u16, i32)> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut seen = Vec::new();
        while seen.len() < count {
            for event in reader.fetch_events().unwrap() {
                if event.event_type() == EventType::KEY {
                    seen.push((event.code(), event.value()));
                }
            }
        }
        let _ = tx.send(seen);
    });
    rx.recv_timeout(READ_TIMEOUT).unwrap()
}

#[test]
#[ignore = "needs write access to /dev/uinput and read access to the new event node; CI grants both"]
fn test_the_device_has_its_name_and_every_key_the_driver_sends() {
    let mut device = open().unwrap();
    let reader = open_reader(&event_node(&mut device));
    assert_eq!(reader.name(), Some(DEVICE_NAME));
    let supported: Vec<u16> = reader.supported_keys().unwrap().iter().map(|key| key.0).collect();
    let mut expected = all_linux_keycodes();
    expected.sort_unstable();
    assert_eq!(supported, expected);
}

#[test]
#[ignore = "needs write access to /dev/uinput and read access to the new event node; CI grants both"]
fn test_keys_sent_to_the_device_arrive_as_key_presses_and_releases() {
    let mut device = open().unwrap();
    let reader = open_reader(&event_node(&mut device));
    device.send(KEY_LEFTSHIFT, true).unwrap();
    device.send(KEY_G, true).unwrap();
    device.send(KEY_G, false).unwrap();
    device.send(KEY_LEFTSHIFT, false).unwrap();
    assert_eq!(
        read_key_events(reader, 4),
        vec![
            (KEY_LEFTSHIFT, KEY_DOWN),
            (KEY_G, KEY_DOWN),
            (KEY_G, KEY_UP),
            (KEY_LEFTSHIFT, KEY_UP)
        ]
    );
}
