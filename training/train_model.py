import torch
import torch.nn as nn
import torch.optim as optim
import torch.onnx
import numpy as np

# 1. Define the Neural Network
# It takes 2 inputs: [RSI, Imbalance]
# It outputs 1 probability: [Buy Confidence] (0.0 = Sell, 1.0 = Buy)
class TradingModel(nn.Module):
    def __init__(self):
        super(TradingModel, self).__init__()
        self.fc1 = nn.Linear(2, 16)
        self.relu = nn.ReLU()
        self.fc2 = nn.Linear(16, 8)
        self.fc3 = nn.Linear(8, 1)
        self.sigmoid = nn.Sigmoid()

    def forward(self, x):
        # x[:, 0] is RSI (0-100). Normalize it to 0-1.
        # x[:, 1] is OBI (-1 to 1). Already normalized.
        rsi = x[:, 0].unsqueeze(1) / 100.0
        obi = x[:, 1].unsqueeze(1)
        
        features = torch.cat((rsi, obi), dim=1)
        
        out = self.fc1(features)
        out = self.relu(out)
        out = self.fc2(out)
        out = self.relu(out)
        out = self.fc3(out)
        return self.sigmoid(out)

# 2. Generate Synthetic Training Data
# We teach the model our "Expert Rules" so it starts smart.
def generate_data(n_samples=10000):
    # Random RSI between 0 and 100
    rsi = np.random.uniform(0, 100, n_samples)
    # Random OBI between -1 and 1
    obi = np.random.uniform(-1, 1, n_samples)
    
    targets = []
    for r, o in zip(rsi, obi):
        if r < 30 and o > 0.2:
            targets.append(1.0) # STRONG BUY
        elif r > 70 and o < -0.2:
            targets.append(0.0) # STRONG SELL
        else:
            targets.append(0.5) # HOLD
            
    X = np.column_stack((rsi, obi)).astype(np.float32)
    y = np.array(targets).astype(np.float32).reshape(-1, 1)
    return torch.from_numpy(X), torch.from_numpy(y)

# 3. Train the Model
def train():
    print("Generating synthetic data...")
    X, y = generate_data()
    
    model = TradingModel()
    criterion = nn.BCELoss()
    optimizer = optim.Adam(model.parameters(), lr=0.01)
    
    print("Training...")
    for epoch in range(500):
        optimizer.zero_grad()
        outputs = model(X)
        loss = criterion(outputs, y)
        loss.backward()
        optimizer.step()
        
        if epoch % 50 == 0:
            print(f"Epoch {epoch}, Loss: {loss.item():.4f}")

    return model

# 4. Export to ONNX
def export_onnx(model, path="models/strategy.onnx"):
    model.eval()
    # Create a dummy input for the exporter: [Batch=1, Features=2]
    dummy_input = torch.randn(1, 2)
    
    print(f"Exporting model to {path}...")
    torch.onnx.export(
        model, 
        dummy_input, 
        path, 
        input_names=['input'], 
        output_names=['output'],
        dynamic_axes={'input': {0: 'batch_size'}, 'output': {0: 'batch_size'}},
        dynamo=False
    )
    print("Success! Model exported.")

if __name__ == "__main__":
    trained_model = train()
    
    # Test a few cases
    test_cases = torch.tensor([
        [20.0, 0.5],  # RSI=20 (Oversold), OBI=0.5 (Buy Pressure) -> Should be close to 1.0
        [80.0, -0.5], # RSI=80 (Overbought), OBI=-0.5 (Sell Pressure) -> Should be close to 0.0
        [50.0, 0.0],  # Neutral -> Should be close to 0.5
    ])
    print("\nTest Predictions:")
    with torch.no_grad():
        preds = trained_model(test_cases)
        for inputs, p in zip(test_cases, preds):
            print(f"Input: RSI={inputs[0]:.1f}, OBI={inputs[1]:.1f} -> Prediction: {p.item():.4f}")

    export_onnx(trained_model)
