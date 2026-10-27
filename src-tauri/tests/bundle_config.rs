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
