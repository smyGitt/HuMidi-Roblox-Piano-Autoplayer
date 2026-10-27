use super::*;
use std::cell::RefCell;

const DMABUF: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

fn run(platform: Platform, already_set: &[&str]) -> Vec<(String, String)> {
    let set_calls = RefCell::new(Vec::new());
    apply_defaults(
        platform,
        |name| already_set.contains(&name),
        |name, value| set_calls.borrow_mut().push((name.to_string(), value.to_string())),
    );
    set_calls.into_inner()
}

mod test_apply_defaults {
    use super::*;

    #[test]
    fn test_on_linux_the_dmabuf_renderer_is_turned_off() {
        assert_eq!(run(Platform::Linux, &[]), vec![(DMABUF.to_string(), "1".to_string())]);
    }

    #[test]
    fn test_a_value_the_user_already_set_is_left_alone() {
        assert!(run(Platform::Linux, &[DMABUF]).is_empty());
    }

    #[test]
    fn test_an_unrelated_variable_being_set_does_not_stop_it() {
        assert_eq!(
            run(Platform::Linux, &["WEBKIT_DISABLE_COMPOSITING_MODE"]),
            vec![(DMABUF.to_string(), "1".to_string())]
        );
    }

    #[test]
    fn test_other_platforms_are_not_touched() {
        assert!(run(Platform::Windows, &[]).is_empty());
        assert!(run(Platform::MacOs, &[]).is_empty());
        assert!(run(Platform::Other, &[]).is_empty());
    }

    #[test]
    fn test_the_defaults_list_names_only_the_dmabuf_renderer() {
        assert_eq!(DEFAULTS, [(DMABUF, "1")]);
    }
}

mod test_apply_process_defaults {
    use super::*;

    #[test]
    fn test_the_process_environment_gets_the_default_only_on_linux_and_only_if_unset() {
        let previous = std::env::var_os(DMABUF);
        std::env::remove_var(DMABUF);
        apply_process_defaults();
        let after_first = std::env::var_os(DMABUF);
        std::env::set_var(DMABUF, "0");
        apply_process_defaults();
        let after_user_value = std::env::var_os(DMABUF);
        match previous {
            Some(value) => std::env::set_var(DMABUF, value),
            None => std::env::remove_var(DMABUF),
        }
        if cfg!(target_os = "linux") {
            assert_eq!(after_first.as_deref(), Some(std::ffi::OsStr::new("1")));
        } else {
            assert_eq!(after_first, None);
        }
        assert_eq!(after_user_value.as_deref(), Some(std::ffi::OsStr::new("0")));
    }
}
