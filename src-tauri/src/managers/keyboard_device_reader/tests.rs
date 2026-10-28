use super::*;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, InputEvent};
use std::sync::mpsc;
use std::sync::{Mutex, MutexGuard};

const TEST_DEVICE_NAME: &str = "HuMidi hook test keyboard";
const KEY_ENTER: u16 = 28;
const KEY_A: u16 = 30;
const KEY_F6: u16 = 64;
const KEY_DOWN: i32 = 1;
const KEY_UP: i32 = 0;
const SCAN_ATTEMPTS: u32 = 100;
const SCAN_RETRY: Duration = Duration::from_millis(100);
const RECEIVE_TIMEOUT: Duration = Duration::from_secs(10);

static ONE_TEST_AT_A_TIME: Mutex<()> = Mutex::new(());

fn one_test_at_a_time() -> MutexGuard<'static, ()> {
    ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

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

fn scan_until_a_device_is_read(sink: &KeySink) {
    for _ in 0..SCAN_ATTEMPTS {
        if scan_once(sink) > 0 {
            return;
        }
        std::thread::sleep(SCAN_RETRY);
    }
    panic!("the new keyboard was never opened for reading");
}

#[test]
#[ignore = "needs write access to /dev/uinput and read access to the new event node; CI grants both"]
fn test_keys_pressed_on_a_keyboard_device_reach_the_sink_and_the_device_is_counted() {
    let _alone = one_test_at_a_time();
    let mut keyboard = test_keyboard();
    let (sender, receiver) = mpsc::channel();
    let sender = Mutex::new(sender);
    let sink: KeySink = Arc::new(move |raw, pressed| {
        let _ = sender.lock().unwrap().send((raw, pressed));
    });

    scan_until_a_device_is_read(&sink);
    assert!(ACTIVE_DEVICES.count() >= 1);

    keyboard.emit(&[InputEvent::new(EventType::KEY.0, KEY_F6, KEY_DOWN)]).unwrap();
    keyboard.emit(&[InputEvent::new(EventType::KEY.0, KEY_F6, KEY_UP)]).unwrap();

    assert_eq!(receiver.recv_timeout(RECEIVE_TIMEOUT), Ok((RawKey::Named("F6"), true)));
    assert_eq!(receiver.recv_timeout(RECEIVE_TIMEOUT), Ok((RawKey::Named("F6"), false)));
}

#[test]
#[ignore = "needs write access to /dev/uinput and read access to the new event node; CI grants both"]
fn test_a_device_that_is_already_read_is_not_read_twice() {
    let _alone = one_test_at_a_time();
    let _keyboard = test_keyboard();
    let sink: KeySink = Arc::new(|_, _| {});
    scan_until_a_device_is_read(&sink);
    let counted = ACTIVE_DEVICES.count();
    assert_eq!(scan_once(&sink), 0);
    assert_eq!(scan_once(&sink), 0);
    assert_eq!(ACTIVE_DEVICES.count(), counted);
}
