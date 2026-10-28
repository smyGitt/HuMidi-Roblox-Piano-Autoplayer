const SIGN_AD_HOC: &str = "-";

fn bundle() -> serde_json::Value {
    let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    config["bundle"].clone()
}

#[test]
fn the_macos_app_is_signed_ad_hoc_so_apple_silicon_does_not_call_it_damaged() {
    assert_eq!(bundle()["macOS"]["signingIdentity"], SIGN_AD_HOC);
}

#[test]
fn the_macos_app_and_dmg_are_still_built() {
    let targets = bundle()["targets"].clone();
    for target in ["app", "dmg"] {
        assert!(targets.as_array().unwrap().iter().any(|t| t == target), "{target}");
    }
}

#[test]
fn the_package_description_is_the_app_description_not_the_template_one() {
    let line = include_str!("../Cargo.toml").lines().find(|l| l.starts_with("description")).unwrap();
    assert_eq!(line, "description = \"Piano MIDI autoplayer for Roblox\"");
}

fn windows(config: &str) -> Vec<serde_json::Value> {
    let config: serde_json::Value = serde_json::from_str(config).unwrap();
    config["app"]["windows"].as_array().unwrap().clone()
}

#[test]
fn the_linux_config_turns_the_window_transparency_off() {
    let linux = windows(include_str!("../tauri.linux.conf.json"));
    assert_eq!(linux.len(), 1);
    assert_eq!(linux[0]["transparent"], false);
}

#[test]
fn every_other_platform_keeps_the_window_transparent() {
    let main = windows(include_str!("../tauri.conf.json"));
    assert_eq!(main.len(), 1);
    assert_eq!(main[0]["transparent"], true);
}

#[test]
fn the_linux_window_has_the_same_title_and_size_as_the_main_one() {
    let main = windows(include_str!("../tauri.conf.json"));
    let linux = windows(include_str!("../tauri.linux.conf.json"));
    for key in ["title", "width", "height"] {
        assert_eq!(linux[0][key], main[0][key], "{key}");
    }
}

fn merged_window_transparency(target: tauri_utils::platform::Target) -> serde_json::Value {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let (merged, _) = tauri_utils::config::parse::read_from(target, root).unwrap();
    merged["app"]["windows"][0]["transparent"].clone()
}

#[test]
fn tauri_itself_merges_the_linux_config_so_only_linux_loses_transparency() {
    use tauri_utils::platform::Target;
    assert_eq!(merged_window_transparency(Target::Linux), false);
    assert_eq!(merged_window_transparency(Target::Windows), true);
    assert_eq!(merged_window_transparency(Target::MacOS), true);
}

const RULES_DESTINATION: &str = "/usr/lib/udev/rules.d/70-humidi-input.rules";
const MODULES_DESTINATION: &str = "/usr/lib/modules-load.d/humidi.conf";
const PACKAGE_FORMATS: [&str; 2] = ["deb", "rpm"];

fn linux_package(format: &str) -> serde_json::Value {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let (merged, _) = tauri_utils::config::parse::read_from(tauri_utils::platform::Target::Linux, root).unwrap();
    merged["bundle"]["linux"][format].clone()
}

fn packaged_file(relative: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

#[test]
fn both_linux_packages_install_the_udev_rules_and_the_module_list() {
    for format in PACKAGE_FORMATS {
        let files = &linux_package(format)["files"];
        assert!(files[RULES_DESTINATION].is_string(), "{format} rules");
        assert!(files[MODULES_DESTINATION].is_string(), "{format} modules");
    }
}

#[test]
fn every_file_the_linux_packages_install_exists_and_is_the_same_for_both() {
    let deb = linux_package("deb");
    let rpm = linux_package("rpm");
    assert_eq!(deb["files"], rpm["files"]);
    for (_, source) in deb["files"].as_object().unwrap() {
        assert!(!packaged_file(source.as_str().unwrap()).is_empty(), "{source}");
    }
}

#[test]
fn the_rules_give_the_logged_in_user_the_uinput_device_and_the_keyboards() {
    let rules = packaged_file(linux_package("deb")["files"][RULES_DESTINATION].as_str().unwrap());
    let lines: Vec<&str> = rules.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 2, "{rules}");
    assert!(lines[0].contains("KERNEL==\"uinput\"") && lines[0].contains("TAG+=\"uaccess\""), "{}", lines[0]);
    assert!(
        lines[1].contains("SUBSYSTEM==\"input\"")
            && lines[1].contains("ENV{ID_INPUT_KEYBOARD}==\"1\"")
            && lines[1].contains("TAG+=\"uaccess\""),
        "{}",
        lines[1]
    );
}

#[test]
fn the_uinput_module_is_loaded_at_boot() {
    let modules = packaged_file(linux_package("rpm")["files"][MODULES_DESTINATION].as_str().unwrap());
    assert_eq!(modules.trim(), "uinput");
}

#[test]
fn the_install_script_loads_the_module_and_reloads_udev_without_ever_failing_the_install() {
    for format in PACKAGE_FORMATS {
        let script = packaged_file(linux_package(format)["postInstallScript"].as_str().unwrap());
        assert!(script.starts_with("#!/bin/sh
"), "{format}");
        assert!(script.contains("modprobe uinput"), "{format}");
        assert!(script.contains("udevadm control --reload-rules"), "{format}");
        assert!(script.contains("udevadm trigger"), "{format}");
        assert!(script.trim_end().ends_with("exit 0"), "{format}");
        assert!(script.lines().filter(|l| l.contains("modprobe") || l.contains("udevadm")).all(|l| l.contains("|| true")), "{format}");
    }
}

#[test]
fn the_remove_script_reloads_udev_without_ever_failing_the_removal() {
    for format in PACKAGE_FORMATS {
        let script = packaged_file(linux_package(format)["postRemoveScript"].as_str().unwrap());
        assert!(script.starts_with("#!/bin/sh
"), "{format}");
        assert!(script.contains("udevadm control --reload-rules"), "{format}");
        assert!(script.trim_end().ends_with("exit 0"), "{format}");
    }
}

#[test]
fn the_packaging_files_have_unix_line_endings_so_the_scripts_run() {
    let mut paths: Vec<String> = Vec::new();
    for format in PACKAGE_FORMATS {
        let package = linux_package(format);
        paths.extend(package["files"].as_object().unwrap().values().map(|v| v.as_str().unwrap().to_string()));
        paths.push(package["postInstallScript"].as_str().unwrap().to_string());
        paths.push(package["postRemoveScript"].as_str().unwrap().to_string());
    }
    for path in paths {
        assert!(!packaged_file(&path).contains('\r'), "{path}");
    }
}

#[test]
fn only_the_linux_config_carries_the_linux_packaging() {
    let main: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    assert!(main["bundle"]["linux"].is_null());
}

#[test]
fn git_checks_the_packaging_files_out_with_unix_line_endings_on_every_system() {
    let attributes = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.gitattributes")).unwrap();
    assert!(
        attributes.lines().any(|l| l.trim() == "src-tauri/packaging/linux/* text eol=lf"),
        "{attributes}"
    );
}
