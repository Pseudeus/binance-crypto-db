To build a "Grade A" Quantitative Dataset capable of predicting across multiple time horizons (Scalping 1m to Swing 1w), you must move beyond simple
  Price/Volume technical analysis. In crypto, the "Alpha" (predictive signal) hides in the Derivatives Market, Liquidity Structure, and Correlations.

  Here is the architectural blueprint for the Ultimate Dataset, categorized by "Information Edge".

  1. The "Hidden Hand": Derivatives Data (Critical for Crypto)
  Crypto spot prices are heavily manipulated/driven by the Perpetual Futures market.
   * Funding Rates: The cost of holding a position.
       * Signal: High positive funding = Overleveraged Longs (Expect mean reversion/dump). High negative = Overleveraged Shorts (Expect short squeeze).
   * Open Interest (OI): The total number of active contracts.
       * Signal: Rising Price + Rising OI = Strong Trend. Rising Price + Falling OI = Weakening Trend (Short covering).
   * Liquidations (`forceOrder`): Real-time forced buy/sell orders.
       * Signal: A cascade of Long Liquidations usually marks a local bottom (V-shape recovery).

   Implementation: Add wss://fstream.binance.com streams to your Ingest Service.

  2. Microstructure: Advanced Order Book Dynamics
  You currently have OBI (Top of Book). You need deeper visibility.
   * Bid-Ask Spread: (BestAsk - BestBid). Widening spread = Uncertainty/Volatility incoming.
   * Market Depth Ratio (L2): Instead of just top 1, sum the volume for top 5, 10, and 20 levels.
       * Feature: (SumBids_20 - SumAsks_20) / Total.
   * Trade Flow Imbalance (TFI) Decay: You added TFI, but calculate it over multiple windows (1m, 5m, 1h) to see if buying pressure is accelerating or
     fading.

  3. "Beta" & Correlations (The Market Context)
  A coin rarely moves alone. It moves with Bitcoin or its Sector (e.g., AI coins, Meme coins).
   * BTC Correlation: Calculate the rolling correlation (Pearson) between TargetSymbol and BTCUSDT.
       * Signal: If BTC dumps and SOL stays flat (correlation drops), SOL is showing "Relative Strength" and is a prime buy candidate when BTC stabilizes.
   * Sector Index: Create a synthetic index for "Meme Coins" (DOGE + SHIB + PEPE). Compare individual coin performance against the index.

  4. Time-Series Encoding (Cyclicality)
  Markets behave differently on Sunday night vs. Monday morning.
   * Sin/Cos Time: Don't use "Hour 0-23". Use Sine/Cosine transformations to tell the model that Hour 23 is close to Hour 0.
       * Hour_Sin = sin(2 * pi * hour / 24)
       * Hour_Cos = cos(2 * pi * hour / 24)

  ---

  The "Golden" Dataset Schema (The Tensor)

  For a specific symbol (e.g., SOLUSDT) at time t, your feature vector should look like this:


  ┌────────────────┬────────────────────────────────────────────────────────┬──────────────────────────┐
  │ Feature Group  │ Features                                               │ Time Windows (Lookbacks) │
  ├────────────────┼────────────────────────────────────────────────────────┼──────────────────────────┤
  │ Momentum       │ RSI, MACD Histogram, ROC (Rate of Change)              │ 1m, 5m, 1h, 4h           │
  │ Microstructure │ OBI (Top), Depth Ratio (L20), TFI (Trade Flow), Spread │ 1m, 5m                   │
  │ Derivatives    │ Funding Rate, OI Change %, Net Liquidations (USD)      │ 1h, 8h                   │
  │ Volatility     │ ATR (Average True Range), Bollinger Band Width         │ 15m, 1h                  │
  │ Correlation    │ Beta to BTC, Relative Strength vs ETH                  │ 1h, 24h                  │
  │ Context        │ Day_Sin, Day_Cos, Week_Sin, Week_Cos                       │ N/A                      │
  └────────────────┴────────────────────────────────────────────────────────┴──────────────────────────┘

  The "Triple Barrier" Labeling Method (Crucial for Training)

  Do not just predict "Price next candle". That is noise. Use the Triple Barrier Method to generate labels for your training data.

  For each data point, set 3 barriers:
   1. Upper Barrier (Take Profit): e.g., +1.0%
   2. Lower Barrier (Stop Loss): e.g., -0.5%
   3. Vertical Barrier (Time Limit): e.g., 60 minutes.

  Label Logic:
   * Class 1 (Buy): Price hits Upper Barrier first.
   * Class 2 (Sell): Price hits Lower Barrier first.
   * Class 0 (Hold/Neutral): Price hits Vertical Barrier (Time Limit) without touching others.

  This teaches the model to find high-probability setups with defined risk/reward, not just "where will price be in 1 minute".

  ---

  Implementation Plan

   1. Ingest Upgrade: Create a FuturesService in Rust to stream Funding/OI/Liquidation data.
   2. Feature Expansion: Update StrategyService to track a "Global State" (BTC Price) to calculate correlations for other coins.
   3. Training Upgrade: Update train_real_data.py to implement Triple Barrier Labeling.

  Recommendation: Since you have 5 days of data, that is enough for 1m/5m/1h predictions. It is insufficient for 1w predictions. For weekly predictions,
  you typically need 2-4 years of data to capture market cycles. Focus on the 1m-24h horizons first.
