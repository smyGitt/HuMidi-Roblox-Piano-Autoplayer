use super::*;
use std::cell::RefCell;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const MINIMUM: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Set(WindowSize),
    Hold,
}

struct FakeWindow {
    size: Result<WindowSize, String>,
    maximized: Result<bool, String>,
    fullscreen: Result<bool, String>,
    failing_set: Option<usize>,
    log: RefCell<Vec<Event>>,
    set_calls: RefCell<usize>,
}

impl FakeWindow {
    fn of(width: u32, height: u32) -> Self {
        FakeWindow {
            size: Ok(size(width, height)),
            maximized: Ok(false),
            fullscreen: Ok(false),
            failing_set: None,
            log: RefCell::new(Vec::new()),
            set_calls: RefCell::new(0),
        }
    }

    fn events(&self) -> Vec<Event> {
        self.log.borrow().clone()
    }
}

impl NudgeTarget for FakeWindow {
    fn inner_size(&self) -> Result<WindowSize, String> {
        self.size.clone()
    }
    fn is_maximized(&self) -> Result<bool, String> {
        self.maximized.clone()
    }
    fn is_fullscreen(&self) -> Result<bool, String> {
        self.fullscreen.clone()
    }
    fn set_inner_size(&self, size: WindowSize) -> Result<(), String> {
        self.log.borrow_mut().push(Event::Set(size));
        let call = *self.set_calls.borrow();
        *self.set_calls.borrow_mut() += 1;
        if self.failing_set == Some(call) {
            return Err(format!("set {call} failed"));
        }
        Ok(())
    }
}

fn size(width: u32, height: u32) -> WindowSize {
    WindowSize { width, height }
}

fn run(platform: Platform, window: &FakeWindow) -> Result<(), String> {
    nudge(platform, window, || window.log.borrow_mut().push(Event::Hold))
}

mod test_nudge {
    use super::*;

    #[test]
    fn test_on_linux_the_width_grows_by_one_pixel_and_the_exact_size_comes_back() {
        let window = FakeWindow::of(WIDTH, HEIGHT);
        run(Platform::Linux, &window).unwrap();
        assert_eq!(
            window.events(),
            vec![
                Event::Set(size(WIDTH + NUDGE_PIXELS, HEIGHT)),
                Event::Hold,
                Event::Set(size(WIDTH, HEIGHT)),
            ]
        );
    }

    #[test]
    fn test_a_window_at_the_smallest_size_still_grows_and_comes_back() {
        let window = FakeWindow::of(MINIMUM, MINIMUM);
        run(Platform::Linux, &window).unwrap();
        assert_eq!(
            window.events(),
            vec![
                Event::Set(size(MINIMUM + NUDGE_PIXELS, MINIMUM)),
                Event::Hold,
                Event::Set(size(MINIMUM, MINIMUM)),
            ]
        );
    }

    #[test]
    fn test_the_largest_possible_width_does_not_overflow_and_is_restored() {
        let window = FakeWindow::of(u32::MAX, HEIGHT);
        run(Platform::Linux, &window).unwrap();
        assert_eq!(window.events().last(), Some(&Event::Set(size(u32::MAX, HEIGHT))));
    }

    #[test]
    fn test_other_platforms_are_not_touched() {
        for platform in [Platform::Windows, Platform::MacOs, Platform::Other] {
            let window = FakeWindow::of(WIDTH, HEIGHT);
            run(platform, &window).unwrap();
            assert!(window.events().is_empty(), "{platform:?}");
        }
    }

    #[test]
    fn test_a_maximized_window_is_left_alone() {
        let mut window = FakeWindow::of(WIDTH, HEIGHT);
        window.maximized = Ok(true);
        run(Platform::Linux, &window).unwrap();
        assert!(window.events().is_empty());
    }

    #[test]
    fn test_a_fullscreen_window_is_left_alone() {
        let mut window = FakeWindow::of(WIDTH, HEIGHT);
        window.fullscreen = Ok(true);
        run(Platform::Linux, &window).unwrap();
        assert!(window.events().is_empty());
    }

    #[test]
    fn test_a_size_that_cannot_be_read_is_an_error_and_changes_nothing() {
        let mut window = FakeWindow::of(WIDTH, HEIGHT);
        window.size = Err("no size".to_string());
        assert_eq!(run(Platform::Linux, &window), Err("no size".to_string()));
        assert!(window.events().is_empty());
    }

    #[test]
    fn test_a_maximized_state_that_cannot_be_read_is_an_error_and_changes_nothing() {
        let mut window = FakeWindow::of(WIDTH, HEIGHT);
        window.maximized = Err("no state".to_string());
        assert_eq!(run(Platform::Linux, &window), Err("no state".to_string()));
        assert!(window.events().is_empty());
    }

    #[test]
    fn test_a_fullscreen_state_that_cannot_be_read_is_an_error_and_changes_nothing() {
        let mut window = FakeWindow::of(WIDTH, HEIGHT);
        window.fullscreen = Err("no state".to_string());
        assert_eq!(run(Platform::Linux, &window), Err("no state".to_string()));
        assert!(window.events().is_empty());
    }

    #[test]
    fn test_when_growing_fails_the_original_size_is_still_set_back_and_the_error_returned() {
        let mut window = FakeWindow::of(WIDTH, HEIGHT);
        window.failing_set = Some(0);
        assert_eq!(run(Platform::Linux, &window), Err("set 0 failed".to_string()));
        assert_eq!(
            window.events(),
            vec![Event::Set(size(WIDTH + NUDGE_PIXELS, HEIGHT)), Event::Set(size(WIDTH, HEIGHT))]
        );
    }

    #[test]
    fn test_when_restoring_fails_the_error_is_returned() {
        let mut window = FakeWindow::of(WIDTH, HEIGHT);
        window.failing_set = Some(1);
        assert_eq!(run(Platform::Linux, &window), Err("set 1 failed".to_string()));
    }
}

mod test_should_nudge {
    use super::*;

    #[test]
    fn test_only_the_main_window_finishing_its_load_is_nudged() {
        assert!(should_nudge(MAIN_WINDOW_LABEL, true));
    }

    #[test]
    fn test_a_load_that_has_only_started_is_not() {
        assert!(!should_nudge(MAIN_WINDOW_LABEL, false));
    }

    #[test]
    fn test_another_window_is_not() {
        assert!(!should_nudge("other", true));
    }
}

mod test_constants {
    use super::*;

    #[test]
    fn test_the_hold_is_shorter_than_the_resize_save_delay_so_only_the_original_size_can_be_saved() {
        const RESIZE_SAVE_DELAY: Duration = Duration::from_millis(400);
        assert!(NUDGE_HOLD < RESIZE_SAVE_DELAY);
    }
}
