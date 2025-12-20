#!/bin/bash
set -e

echo "Installing systemd services..."

# Build release if not exists
if [ ! -f target/release/binance-crypto-db ]; then
    echo "Compiling release binary..."
    cargo build --release
fi

# Copy service files
sudo cp systemd/binance-mock.service /etc/systemd/system/
sudo cp systemd/binance-trader.service /etc/systemd/system/

# Reload daemons
sudo systemctl daemon-reload

# Enable and start
sudo systemctl enable binance-mock.service
sudo systemctl enable binance-trader.service

echo "Services installed and enabled."
echo "Starting Mock Server..."
sudo systemctl start binance-mock.service
echo "Starting Trader..."
sudo systemctl start binance-trader.service

echo "Status:"
systemctl status binance-mock.service --no-pager
systemctl status binance-trader.service --no-pager
