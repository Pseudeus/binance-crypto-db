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

    pub fn predict(&self, features: &[f32]) -> Result<f32, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(model) = &self.model {
            // Create input tensor (1, N)
            let tensor = tract_ndarray::Array::from_shape_vec((1, features.len()), features.to_vec())?
                .into_tensor();

            let result = model.run(tvec!(tensor.into()))?;
            
            // Assuming output is a single probability scalar [Batch, 1] or just [1]
            let output_tensor = result[0].to_array_view::<f32>()?;
            let probability = output_tensor.iter().next().copied().unwrap_or(0.5);
            
            Ok(probability)
        } else {
            // Dummy logic for simulation: 
            // Return 0.5 (Neutral)
            Ok(0.5)
        }
    }
}
