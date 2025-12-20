import torch
import torch.nn as nn
import torch.optim as optim
import torch.onnx
import numpy as np
import pandas as pd
import pandas_ta as ta
import sqlite3
import zstandard as zstd
import os
import subprocess
from tqdm import tqdm
from torch.utils.data import TensorDataset, DataLoader

# ==========================================
# 1. Model Architecture (Multi-Class)
# ==========================================
class TradingModel(nn.Module):
    def __init__(self):
        super(TradingModel, self).__init__()
        # Input: 4 Features (RSI, OBI, TFI, Volatility)
        # Output: 3 Classes (0: HOLD, 1: BUY, 2: SELL)
        self.fc1 = nn.Linear(4, 64)
        self.relu = nn.ReLU()
        self.dropout = nn.Dropout(0.2)
        self.fc2 = nn.Linear(64, 32)
        self.fc3 = nn.Linear(32, 3) # Output logits for 3 classes

    def forward(self, x):
        # Normalization (Simple scaling based on expected ranges)
        rsi = x[:, 0].unsqueeze(1) / 100.0  # 0-100 -> 0-1
        obi = x[:, 1].unsqueeze(1)          # -1 to 1 -> as is
        tfi = x[:, 2].unsqueeze(1)          # -1 to 1 -> as is
        # Volatility usually < 100 for crypto, but can spike. Clamp to safe range.
        vol = torch.clamp(x[:, 3].unsqueeze(1) / 50.0, 0, 1)

        features = torch.cat((rsi, obi, tfi, vol), dim=1)

        out = self.fc1(features)
        out = self.relu(out)
        out = self.dropout(out)
        out = self.fc2(out)
        out = self.relu(out)
        out = self.fc3(out)
        return out # Return Logits (CrossEntropyLoss handles Softmax)

# ==========================================
# 2. Database Utilities
# ==========================================
def load_and_decompress_db(zstd_path, output_db_path="temp_db.sqlite"):
    print(f"Importing {zstd_path} via shell pipeline...")
    if os.path.exists(output_db_path):
        os.remove(output_db_path)
    try:
        cmd = f"zstd -dc '{zstd_path}' | sqlite3 '{output_db_path}'"
        print(f"Executing: {cmd}")
        retcode = subprocess.call(cmd, shell=True)
        if retcode != 0:
            print(f"Shell command failed with exit code {retcode}")
            return None
        if not os.path.exists(output_db_path) or os.path.getsize(output_db_path) == 0:
             print("Output database empty or not created.")
             return None
        print(f"Import complete. SQLite DB ready at {output_db_path}")
        return output_db_path
    except Exception as e:
        print(f"Error during import: {e}")
        return None

def fetch_data_from_db(db_path):
    conn = sqlite3.connect(db_path)
    print("Fetching agg_trades data...")
    aggtrade_df = pd.read_sql_query("SELECT time, symbol, price, quantity, is_buyer_maker FROM agg_trades ORDER BY time;", conn)
    print("Fetching order_books data...")
    orderbook_df = pd.read_sql_query("SELECT time, symbol, bids, asks FROM order_books ORDER BY time;", conn)
    conn.close()

    if 'time' in aggtrade_df.columns:
        aggtrade_df['time'] = pd.to_datetime(aggtrade_df['time'], unit='s')
    if 'time' in orderbook_df.columns:
        orderbook_df['time'] = pd.to_datetime(orderbook_df['time'], unit='s')
    return aggtrade_df, orderbook_df

# ==========================================
# 3. Feature Engineering & Labeling
# ==========================================
def calculate_features_and_labels(aggtrade_df, orderbook_df, interval_ms=60000):
    """
    Generates features and implements TRIPLE BARRIER LABELING.
    Returns: X (Features), y (Labels: 0=Hold, 1=Buy, 2=Sell)
    """
    all_features = []
    all_labels = []
    
    # Configuration for Labeling
    BARRIER_HORIZON = 15  # Look ahead 15 bars
    BARRIER_MULTIPLIER = 2.0 # Upper/Lower barrier = Price +/- (Volatility * Multiplier)

    print(f"AggTrade Rows: {len(aggtrade_df)}, OrderBook Rows: {len(orderbook_df)}")
    
    for symbol, agg_group in tqdm(aggtrade_df.groupby('symbol'), desc="Processing Symbols"):
        agg_group = agg_group.set_index('time').sort_index()

        # --- A. OHLCV & Indicators ---
        ohlcv = agg_group['price'].resample(f'{interval_ms}ms').ohlc().dropna()
        if ohlcv.empty: continue

        # 1. RSI
        rsi = ta.rsi(ohlcv['close'], length=14)

        # 2. TFI (Volume Imbalance)
        agg_group['vol_buy'] = np.where(agg_group['is_buyer_maker'] == False, agg_group['quantity'], 0)
        agg_group['vol_sell'] = np.where(agg_group['is_buyer_maker'] == True, agg_group['quantity'], 0)
        vol_resampled = agg_group[['vol_buy', 'vol_sell']].resample(f'{interval_ms}ms').sum()
        total_vol = vol_resampled['vol_buy'] + vol_resampled['vol_sell']
        tfi = (vol_resampled['vol_buy'] - vol_resampled['vol_sell']) / total_vol.replace(0, 1)

        # 3. Volatility (Rolling Std Dev)
        rolling_std = ohlcv['close'].rolling(20).std()

        # 4. OBI (Order Book Imbalance)
        current_symbol_orderbook = orderbook_df[orderbook_df['symbol'] == symbol]
        ob_list = []
        for idx, row in current_symbol_orderbook.iterrows():
            try:
                bids = np.frombuffer(row['bids'], dtype=np.float32).reshape(-1, 2)
                asks = np.frombuffer(row['asks'], dtype=np.float32).reshape(-1, 2)
                bid_vol = bids[:, 1].sum()
                ask_vol = asks[:, 1].sum()
                total = bid_vol + ask_vol
                obi_val = (bid_vol - ask_vol) / total if total > 0 else 0
                ob_list.append({'time': row['time'], 'obi': obi_val})
            except: continue
        
        if not ob_list:
            obi_resampled = pd.Series(0, index=ohlcv.index)
        else:
            obi_df = pd.DataFrame(ob_list).set_index('time').sort_index()
            obi_resampled = obi_df['obi'].resample(f'{interval_ms}ms').last().ffill()
            obi_resampled = obi_resampled.reindex(ohlcv.index, method='ffill').fillna(0)

        # Combine Features
        features_df = pd.DataFrame({
            'rsi': rsi,
            'obi': obi_resampled,
            'tfi': tfi,
            'volatility': rolling_std,
            'close': ohlcv['close'], # kept for labeling, removed later
            'high': ohlcv['high'],   # kept for labeling
            'low': ohlcv['low']      # kept for labeling
        }).dropna()

        if features_df.empty: continue

        # --- B. Triple Barrier Labeling ---
        # We iterate to find the first barrier touch.
        # This is slow in pure Python, but accurate.
        
        closes = features_df['close'].values
        highs = features_df['high'].values
        lows = features_df['low'].values
        vols = features_df['volatility'].values
        
        labels = []
        valid_indices = []

        for i in range(len(features_df) - BARRIER_HORIZON):
            current_price = closes[i]
            current_vol = vols[i]
            
            # Dynamic Barriers
            # If vol is very low, use a minimum floor to avoid noise
            barrier_width = max(current_vol * BARRIER_MULTIPLIER, current_price * 0.002) # Min 0.2% move
            
            upper_barrier = current_price + barrier_width
            lower_barrier = current_price - barrier_width
            
            label = 0 # Default: HOLD
            
            # Look ahead
            for j in range(1, BARRIER_HORIZON + 1):
                future_high = highs[i+j]
                future_low = lows[i+j]
                
                # Check touches
                touched_upper = future_high >= upper_barrier
                touched_lower = future_low <= lower_barrier
                
                if touched_upper and touched_lower:
                    # Touched both in same candle? rare. Assume volatility/chop -> HOLD or check close
                    # For safety, let's call it HOLD (0) or invalid.
                    break 
                elif touched_upper:
                    label = 1 # BUY
                    break
                elif touched_lower:
                    label = 2 # SELL
                    break
            
            labels.append(label)
            valid_indices.append(i)

        # Extract X and y using valid indices
        final_X = features_df.iloc[valid_indices][['rsi', 'obi', 'tfi', 'volatility']].values
        final_y = np.array(labels)
        
        all_features.append(final_X)
        all_labels.append(final_y)

    if not all_features:
        return np.array([]), np.array([])

    X_concat = np.concatenate(all_features, axis=0)
    y_concat = np.concatenate(all_labels, axis=0)
    
    return X_concat, y_concat

# ==========================================
# 4. Balancing & Confusers (Prior Probabilities)
# ==========================================
def balance_dataset(X, y):
    """
    Implements Class Balancing and Confuser Retention.
    Classes: 0 (Hold), 1 (Buy), 2 (Sell)
    Confusers: Class 0 samples with extreme RSI (<30 or >70).
    """
    print(f"Original Distribution: Hold={np.sum(y==0)}, Buy={np.sum(y==1)}, Sell={np.sum(y==2)}")
    
    indices = np.arange(len(y))
    
    idx_buy = indices[y == 1]
    idx_sell = indices[y == 2]
    idx_hold = indices[y == 0]
    
    # Identify Confusers within Hold
    # RSI is column 0.
    rsi_hold = X[idx_hold, 0]
    # Confusers: Hold (0) but RSI < 30 or RSI > 70
    is_confuser = (rsi_hold < 30) | (rsi_hold > 70)
    
    idx_hold_confuser = idx_hold[is_confuser]
    idx_hold_boring = idx_hold[~is_confuser]
    
    print(f"Confusers (Hard Negatives): {len(idx_hold_confuser)}")
    print(f"Boring Holds: {len(idx_hold_boring)}")
    
    # Target count: Match majority of signals (Buy/Sell) * Multiplier
    n_signals = max(len(idx_buy), len(idx_sell))
    if n_signals == 0:
        return X, y # Cannot balance

    # We want to keep ALL Buy, ALL Sell, ALL Confusers
    # And sample Boring Holds to match, say, 1:1 ratio with signals
    n_keep_boring = min(len(idx_hold_boring), n_signals * 2) 
    
    if n_keep_boring > 0:
        idx_hold_boring_selected = np.random.choice(idx_hold_boring, n_keep_boring, replace=False)
    else:
        idx_hold_boring_selected = np.array([], dtype=int)
        
    final_indices = np.concatenate([idx_buy, idx_sell, idx_hold_confuser, idx_hold_boring_selected])
    np.random.shuffle(final_indices)
    
    X_balanced = X[final_indices]
    y_balanced = y[final_indices]
    
    print(f"Balanced Distribution: Hold={np.sum(y_balanced==0)}, Buy={np.sum(y_balanced==1)}, Sell={np.sum(y_balanced==2)}")
    return X_balanced, y_balanced

# ==========================================
# 5. Training Loop
# ==========================================
def train_real(X, y):
    model = TradingModel()
    # Weighted Loss? Since we manually balanced, standard CrossEntropy is fine.
    # But we can add slight weights if still uneven.
    criterion = nn.CrossEntropyLoss() 
    optimizer = optim.Adam(model.parameters(), lr=0.001)
    
    print("Training on balanced data...")
    dataset = TensorDataset(torch.from_numpy(X).float(), torch.from_numpy(y).long())
    dataloader = DataLoader(dataset, batch_size=64, shuffle=True)
    
    for epoch in range(50):
        total_loss = 0
        correct = 0
        total = 0
        
        for batch_X, batch_y in dataloader:
            optimizer.zero_grad()
            outputs = model(batch_X)
            loss = criterion(outputs, batch_y)
            loss.backward()
            optimizer.step()
            
            total_loss += loss.item()
            _, predicted = torch.max(outputs.data, 1)
            total += batch_y.size(0)
            correct += (predicted == batch_y).sum().item()
        
        if epoch % 10 == 0:
            acc = 100 * correct / total
            print(f"Epoch {epoch}, Loss: {total_loss/len(dataloader):.4f}, Acc: {acc:.2f}%")

    return model

def export_onnx(model, path="models/strategy.onnx"):
    model.eval()
    dummy_input = torch.randn(1, 4)
    if not os.path.exists("models"):
        os.makedirs("models")
        
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

# ==========================================
# Main Execution
# ==========================================
if __name__ == "__main__":
    zstd_file_path = os.getenv("COLLECTED_DB_PATH", "collected_data.sql.zstd")
    
    if not os.path.exists(zstd_file_path):
        print(f"File not found: {zstd_file_path}")
        print("Please set COLLECTED_DB_PATH or place 'collected_data.sql.zstd' in this directory.")
    else:
        decompressed_db_path = load_and_decompress_db(zstd_file_path)

        if decompressed_db_path:
            aggtrade_df, orderbook_df = fetch_data_from_db(decompressed_db_path)
            
            # 1. Feature Engineering & Labeling (Parent Distribution)
            X, y = calculate_features_and_labels(aggtrade_df, orderbook_df)
            
            if X.size > 0:
                # 2. Class Balancing & Confusers (Prior Probabilities)
                X_bal, y_bal = balance_dataset(X, y)
                
                # 3. Train
                if X_bal.size > 0:
                    trained_model = train_real(X_bal, y_bal)
                    export_onnx(trained_model)
                else:
                    print("Balanced dataset is empty.")
            else:
                print("No features generated.")

            try:
                os.remove(decompressed_db_path)
                print("Temp DB removed.")
            except: pass