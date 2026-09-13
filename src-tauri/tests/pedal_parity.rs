use humidi_tauri_lib::core::pedal::model::{PedalModel, FEATURES};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn read_f32_bin(path: &PathBuf) -> Vec<f32> {
    let bytes = std::fs::read(path).unwrap();
    bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

#[test]
fn bilstm_matches_pytorch_reference() {
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("pedal_bilstm.safetensors");
    let model = PedalModel::load(model_path).unwrap();

    let input = read_f32_bin(&fixture_path("ref_input.bin"));
    let expected = read_f32_bin(&fixture_path("ref_output.bin"));
    let t_len = expected.len();
    assert_eq!(input.len(), t_len * FEATURES);

    let actual = model.forward(&input, t_len).unwrap();
    assert_eq!(actual.len(), expected.len());

    let max_abs_diff = actual
        .iter()
        .zip(expected.iter())
        .map(|(a, e)| (a - e).abs())
        .fold(0.0f32, f32::max);

    assert!(
        max_abs_diff < 1e-4,
        "max abs diff {max_abs_diff} exceeds tolerance"
    );
}
