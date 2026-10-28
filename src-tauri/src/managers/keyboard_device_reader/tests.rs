use super::*;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, InputEvent};
use std::sync::mpsc;
use std::sync::Mutex;

const TEST_DEVICE_NAME: &str = "HuMidi hook test keyboard";
const KEY_ENTER: u16 = 28;
const KEY_A: u16 = 30;
const KEY_F6: u16 = 64;
const KEY_DOWN: i32 = 1;
const KEY_UP: i32 = 0;
const SCAN_ATTEMPTS: u32 = 100;
const SCAN_RETRY: Duration = Duration::from_millis(100);
const RECEIVE_TIMEOUT: Duration = Duration::from_secs(10);

fn test_keyboard() -> VirtualDevice {
    let mut keys = AttributeSet::<KeyCode>::new();
    for code in [KEY_A, KEY_ENTER, KEY_F6] {
        keys.insert(KeyCode(code));
    }
    VirtualDevice::builder()
        .unwrap()
        .name(TEST_DEVICE_NAME)
        .with_keys(&keys)
        .unwrap()
        .build()
        .unwrap()
}

fn scan_until_a_device_is_read(sink: &KeySink, before: usize) {
    for _ in 0..SCAN_ATTEMPTS {
        scan_once(sink);
        if ACTIVE_DEVICES.count() > before {
            return;
        }
        std::thread::sleep(SCAN_RETRY);
    }
    panic!("the new keyboard was never opened for reading");
}

#[test]
#[ignore = "needs write access to /dev/uinput and read access to the new event node; CI grants both"]
fn test_keys_pressed_on_a_keyboard_device_reach_the_sink_and_the_device_is_counted() {
    let mut keyboard = test_keyboard();
    let (sender, receiver) = mpsc::channel();
    let sender = Mutex::new(sender);
    let sink: KeySink = Arc::new(move |raw, pressed| {
        let _ = sender.lock().unwrap().send((raw, pressed));
    });
    let before = ACTIVE_DEVICES.count();

    scan_until_a_device_is_read(&sink, before);
    assert!(ACTIVE_DEVICES.count() > before);

    keyboard.emit(&[InputEvent::new(EventType::KEY.0, KEY_F6, KEY_DOWN)]).unwrap();
    keyboard.emit(&[InputEvent::new(EventType::KEY.0, KEY_F6, KEY_UP)]).unwrap();

    assert_eq!(receiver.recv_timeout(RECEIVE_TIMEOUT), Ok((RawKey::Named("F6"), true)));
    assert_eq!(receiver.recv_timeout(RECEIVE_TIMEOUT), Ok((RawKey::Named("F6"), false)));
}

#[test]
#[ignore = "needs write access to /dev/uinput and read access to the new event node; CI grants both"]
fn test_a_device_that_is_already_read_is_not_read_twice() {
    let _keyboard = test_keyboard();
    let sink: KeySink = Arc::new(|_, _| {});
    let before = ACTIVE_DEVICES.count();
    scan_until_a_device_is_read(&sink, before);
    let counted = ACTIVE_DEVICES.count();
    scan_once(&sink);
    scan_once(&sink);
    assert_eq!(ACTIVE_DEVICES.count(), counted);
}
