use std::path::Path;
use std::sync::Arc;
use tract_onnx::prelude::*;
use tracing::{debug, error, info, warn};

type RunnableModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

#[derive(Clone)]
pub struct InferenceEngine {
    model: Option<Arc<RunnableModel>>,
}

impl InferenceEngine {
    pub fn new(model_path: &str) -> Self {
        let path = Path::new(model_path);
        let model = if path.exists() {
            info!("Loading ONNX model from {:?}", path);
            match Self::load_model(model_path) {
                Ok(plan) => Some(Arc::new(plan)),
                Err(e) => {
                    error!("Failed to load model: {}", e);
                    None
                }
            }
        } else {
            warn!("ONNX model not found at {:?}. Running in SIMULATION mode (Dummy Predictions).", path);
            None
        };

        Self { model }
    }

    fn load_model(path: &str) -> TractResult<RunnableModel> {
        let model = tract_onnx::onnx()
            .model_for_path(path)?
            .into_optimized()?
            .into_runnable()?;
        Ok(model)
    }

    pub fn predict(&self, features: &[f32]) -> Result<InferenceResult, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(model) = &self.model {
            // Create input tensor (1, N)
            let tensor = tract_ndarray::Array::from_shape_vec((1, features.len()), features.to_vec())?
                .into_tensor();

            let result = model.run(tvec!(tensor.into()))?;
            
            // Output is [1, 3] Logits (Hold, Buy, Sell)
            let logits = result[0].to_array_view::<f32>()?;
            let logits_slice = logits.as_slice().ok_or("Failed to get logits slice")?;

            // Softmax
            let max_logit = logits_slice.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let exp_sum: f32 = logits_slice.iter().map(|&x| (x - max_logit).exp()).sum();
            let probs: Vec<f32> = logits_slice.iter().map(|&x| (x - max_logit).exp() / exp_sum).collect();

            // ArgMax
            let mut max_index = 0;
            let mut max_prob = 0.0;
            for (i, &prob) in probs.iter().enumerate() {
                if prob > max_prob {
                    max_prob = prob;
                    max_index = i;
                }
            }

            Ok(InferenceResult {
                class: max_index,
                confidence: max_prob,
            })
        } else {
            // Dummy logic for simulation
            Ok(InferenceResult { class: 0, confidence: 0.0 })
        }
    }
}

#[derive(Debug, Clone)]
pub struct InferenceResult {
    pub class: usize, // 0=Hold, 1=Buy, 2=Sell
    pub confidence: f32,
}
