use super::*;
use serde_json::json;

mod test_parse_version {
    use super::*;

    #[test]
    fn test_two_digit_tag() {
        assert_eq!(parse_version("v1.3"), vec![1, 3]);
    }

    #[test]
    fn test_three_digit_tag() {
        assert_eq!(parse_version("v1.3.0"), vec![1, 3, 0]);
    }

    #[test]
    fn test_no_v_prefix() {
        assert_eq!(parse_version("2.1"), vec![2, 1]);
    }

    #[test]
    fn test_garbage_returns_empty() {
        assert_eq!(parse_version("garbage"), Vec::<i64>::new());
    }

    #[test]
    fn test_empty_string_returns_empty() {
        assert_eq!(parse_version(""), Vec::<i64>::new());
    }

    #[test]
    fn test_newer_version_compares_greater() {
        assert!(parse_version("v1.4") > parse_version("v1.3"));
    }

    #[test]
    fn test_major_bump_compares_greater() {
        assert!(parse_version("v2.0") > parse_version("v1.9"));
    }

    #[test]
    fn test_same_version_equal() {
        assert_eq!(parse_version("v1.3"), parse_version("v1.3"));
    }

    #[test]
    fn test_multiple_leading_v_all_stripped() {
        assert_eq!(parse_version("vv1.3"), vec![1, 3]);
    }

    #[test]
    fn test_partial_garbage_segment_returns_empty() {
        assert_eq!(parse_version("v1.x.0"), Vec::<i64>::new());
    }

    #[test]
    fn test_leading_whitespace_before_v_is_not_stripped() {
        assert_eq!(parse_version("  v1.3  "), Vec::<i64>::new());
    }

    #[test]
    fn test_trailing_whitespace_after_v_prefix_is_trimmed() {
        assert_eq!(parse_version("v1.3  "), vec![1, 3]);
    }
}

mod test_evaluate_update {
    use super::*;

    #[test]
    fn test_update_available_when_latest_is_newer() {
        let outcome = evaluate_update("1.3", "v1.4");
        assert_eq!(
            outcome,
            UpdateCheckOutcome::UpdateAvailable {
                tag: "v1.4".to_string(),
                url: RELEASES_PAGE.to_string(),
            }
        );
    }

    #[test]
    fn test_no_update_when_version_is_current() {
        assert_eq!(evaluate_update("1.3", "v1.3"), UpdateCheckOutcome::NoUpdate);
    }

    #[test]
    fn test_update_available_for_major_version_bump() {
        let outcome = evaluate_update("1.3", "v2.1");
        assert_eq!(
            outcome,
            UpdateCheckOutcome::UpdateAvailable {
                tag: "v2.1".to_string(),
                url: RELEASES_PAGE.to_string(),
            }
        );
    }

    #[test]
    fn test_no_update_when_local_is_newer() {
        assert_eq!(evaluate_update("1.4", "v1.3"), UpdateCheckOutcome::NoUpdate);
    }

    #[test]
    fn test_indeterminate_on_missing_tag_name() {
        assert_eq!(evaluate_update("1.3", ""), UpdateCheckOutcome::Indeterminate);
    }

    #[test]
    fn test_indeterminate_on_unparseable_latest_tag() {
        assert_eq!(
            evaluate_update("1.3", "garbage"),
            UpdateCheckOutcome::Indeterminate
        );
    }

    #[test]
    fn test_indeterminate_on_unparseable_current_version() {
        assert_eq!(
            evaluate_update("garbage", "v1.4"),
            UpdateCheckOutcome::Indeterminate
        );
    }
}

mod test_latest_tag_from_release {
    use super::*;

    #[test]
    fn test_reads_tag_name() {
        assert_eq!(latest_tag_from_release(&json!({"tag_name": "v2.2.0"})), "v2.2.0");
    }

    #[test]
    fn test_missing_tag_name_is_empty() {
        assert_eq!(latest_tag_from_release(&json!({"name": "x"})), "");
    }

    #[test]
    fn test_non_string_tag_name_is_empty() {
        assert_eq!(latest_tag_from_release(&json!({"tag_name": 5})), "");
    }

    #[test]
    fn test_a_not_found_body_leads_to_indeterminate() {
        let body = json!({"message": "Not Found"});
        assert_eq!(
            evaluate_update("2.1.6", &latest_tag_from_release(&body)),
            UpdateCheckOutcome::Indeterminate
        );
    }

    #[test]
    fn test_a_newer_release_body_leads_to_update_available() {
        let body = json!({"tag_name": "v2.2.0"});
        assert_eq!(
            evaluate_update("2.1.6", &latest_tag_from_release(&body)),
            UpdateCheckOutcome::UpdateAvailable {
                tag: "v2.2.0".to_string(),
                url: RELEASES_PAGE.to_string()
            }
        );
    }
}

mod test_auto_check_enabled {
    use super::*;

    #[test]
    fn test_default_false_when_key_absent() {
        assert!(!auto_check_enabled(&json!({"other": 1})));
    }

    #[test]
    fn test_default_false_when_config_empty() {
        assert!(!auto_check_enabled(&json!({})));
    }

    #[test]
    fn test_respects_explicit_true() {
        assert!(auto_check_enabled(&json!({"auto_check_updates": true})));
    }

    #[test]
    fn test_respects_explicit_false() {
        assert!(!auto_check_enabled(&json!({"auto_check_updates": false})));
    }
}
