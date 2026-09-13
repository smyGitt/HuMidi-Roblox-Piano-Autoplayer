use candle_core::{DType, Device, Result, Tensor};
use std::collections::HashMap;
use std::path::Path;

const HIDDEN: usize = 256;
pub const FEATURES: usize = 140;
const CHUNK: usize = 1024;
const STRIDE: usize = 512;

struct LstmWeights {
    w_ih: Tensor,
    w_hh: Tensor,
    b_ih: Tensor,
    b_hh: Tensor,
}

pub struct PedalModel {
    tensors: HashMap<String, Tensor>,
    device: Device,
}

fn sigmoid(x: &Tensor) -> Result<Tensor> {
    let ones = Tensor::ones_like(x)?;
    (ones + x.neg()?.exp()?)?.recip()
}

fn load_lstm_weights(tensors: &HashMap<String, Tensor>, layer: usize, reverse: bool) -> LstmWeights {
    let suffix = if reverse { "_reverse" } else { "" };
    let get = |name: String| tensors.get(&name).unwrap().clone();
    LstmWeights {
        w_ih: get(format!("lstm.weight_ih_l{layer}{suffix}")),
        w_hh: get(format!("lstm.weight_hh_l{layer}{suffix}")),
        b_ih: get(format!("lstm.bias_ih_l{layer}{suffix}")),
        b_hh: get(format!("lstm.bias_hh_l{layer}{suffix}")),
    }
}

fn lstm_direction(x_seq: &[Tensor], weights: &LstmWeights, device: &Device) -> Result<Vec<Tensor>> {
    let mut h = Tensor::zeros((1, HIDDEN), DType::F32, device)?;
    let mut c = Tensor::zeros((1, HIDDEN), DType::F32, device)?;
    let w_ih_t = weights.w_ih.t()?;
    let w_hh_t = weights.w_hh.t()?;
    let mut outputs = Vec::with_capacity(x_seq.len());

    for x_t in x_seq {
        let gates_x = x_t.matmul(&w_ih_t)?.broadcast_add(&weights.b_ih)?;
        let gates_h = h.matmul(&w_hh_t)?.broadcast_add(&weights.b_hh)?;
        let gates = (gates_x + gates_h)?;

        let i_gate = sigmoid(&gates.narrow(1, 0 * HIDDEN, HIDDEN)?)?;
        let f_gate = sigmoid(&gates.narrow(1, 1 * HIDDEN, HIDDEN)?)?;
        let g_gate = gates.narrow(1, 2 * HIDDEN, HIDDEN)?.tanh()?;
        let o_gate = sigmoid(&gates.narrow(1, 3 * HIDDEN, HIDDEN)?)?;

        c = ((f_gate * &c)? + (i_gate * g_gate)?)?;
        h = (o_gate * c.tanh()?)?;
        outputs.push(h.clone());
    }
    Ok(outputs)
}

fn bilstm_layer(x_seq: &[Tensor], fwd: &LstmWeights, bwd: &LstmWeights, device: &Device) -> Result<Vec<Tensor>> {
    let fwd_out = lstm_direction(x_seq, fwd, device)?;
    let reversed: Vec<Tensor> = x_seq.iter().rev().cloned().collect();
    let mut bwd_out = lstm_direction(&reversed, bwd, device)?;
    bwd_out.reverse();

    fwd_out
        .iter()
        .zip(bwd_out.iter())
        .map(|(f, b)| Tensor::cat(&[f, b], 1))
        .collect()
}

impl PedalModel {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let device = Device::Cpu;
        let tensors = candle_core::safetensors::load(path, &device)?;
        Ok(PedalModel { tensors, device })
    }

    fn chunk_forward(&self, seq: &[f32], t_len: usize) -> Result<Vec<f32>> {
        let input = Tensor::from_slice(seq, (t_len, FEATURES), &self.device)?;
        let x_seq: Vec<Tensor> = (0..t_len)
            .map(|t| input.narrow(0, t, 1))
            .collect::<Result<Vec<_>>>()?;

        let l0_fwd = load_lstm_weights(&self.tensors, 0, false);
        let l0_bwd = load_lstm_weights(&self.tensors, 0, true);
        let layer0_out = bilstm_layer(&x_seq, &l0_fwd, &l0_bwd, &self.device)?;

        let l1_fwd = load_lstm_weights(&self.tensors, 1, false);
        let l1_bwd = load_lstm_weights(&self.tensors, 1, true);
        let layer1_out = bilstm_layer(&layer0_out, &l1_fwd, &l1_bwd, &self.device)?;

        let head_w = self.tensors.get("regression_head.weight").unwrap().t()?;
        let head_b = self.tensors.get("regression_head.bias").unwrap().clone();

        let mut preds = Vec::with_capacity(t_len);
        for h_t in &layer1_out {
            let logit = h_t.matmul(&head_w)?.broadcast_add(&head_b)?;
            let p = sigmoid(&logit)?;
            preds.push(p.to_vec2::<f32>()?[0][0]);
        }
        Ok(preds)
    }

    pub fn forward(&self, seq: &[f32], t_len: usize) -> Result<Vec<f32>> {
        if t_len <= CHUNK {
            return self.chunk_forward(seq, t_len);
        }

        let mut preds = vec![0.0f32; t_len];
        let mut counts = vec![0.0f32; t_len];

        let mut start = 0usize;
        while start < t_len {
            let end = (start + CHUNK).min(t_len);
            let chunk_len = end - start;
            let chunk_slice = &seq[start * FEATURES..end * FEATURES];

            let chunk_preds = if chunk_len < CHUNK {
                let mut padded = chunk_slice.to_vec();
                padded.resize(CHUNK * FEATURES, 0.0);
                let full_preds = self.chunk_forward(&padded, CHUNK)?;
                full_preds[..chunk_len].to_vec()
            } else {
                self.chunk_forward(chunk_slice, chunk_len)?
            };

            let center = chunk_preds.len() as f32 / 2.0;
            for (i, p) in chunk_preds.iter().enumerate() {
                let weight = 1.0 - (i as f32 - center).abs() / center;
                preds[start + i] += p * weight;
                counts[start + i] += weight;
            }

            start += STRIDE;
        }

        for i in 0..t_len {
            preds[i] /= counts[i];
        }
        Ok(preds)
    }
}
