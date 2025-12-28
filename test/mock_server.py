import http.server
import socketserver
import json
import time
import urllib.parse

PORT = 8080

class BinanceMockHandler(http.server.SimpleHTTPRequestHandler):
    def do_POST(self):
        # Handle /api/v3/order
        if self.path.startswith("/api/v3/order"):
            # 1. Parse Query Params from URL
            parsed_url = urllib.parse.urlparse(self.path)
            query_params = urllib.parse.parse_qs(parsed_url.query)
            
            # 2. Parse Body Params (if any)
            content_length = self.headers.get('Content-Length')
            body_params = {}
            if content_length:
                length = int(content_length)
                if length > 0:
                    post_data = self.rfile.read(length).decode('utf-8')
                    body_params = urllib.parse.parse_qs(post_data)
            
            # Merge params (Body overrides URL if conflict, though usually they shouldn't conflict)
            # Flatten lists from parse_qs (it returns {'key': ['val']})
            def get_param(key, default):
                if key in body_params:
                    return body_params[key][0]
                if key in query_params:
                    return query_params[key][0]
                return default

            symbol = get_param('symbol', 'UNKNOWN')
            side = get_param('side', 'UNKNOWN')
            qty = get_param('quantity', '0')
            
            print(f"\n[MOCK BINANCE] RECEIVED ORDER: {side} {qty} {symbol}")
            
            # Simulate a successful fill response
            response = {
                "symbol": symbol,
                "orderId": int(time.time() * 1000),
                "orderListId": -1,
                "clientOrderId": "mock_id",
                "transactTime": int(time.time() * 1000),
                "price": "0.00000000", # Market order
                "origQty": qty,
                "executedQty": qty,
                "cummulativeQuoteQty": "100.00000000", # Fake USDT amount
                "status": "FILLED",
                "timeInForce": "GTC",
                "type": "MARKET",
                "side": side
            }
            
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps(response).encode('utf-8'))
            
    def do_GET(self):
        # Handle /api/v3/account (Balance check)
        if self.path.startswith("/api/v3/account"):
            print("\n[MOCK BINANCE] RECEIVED ACCOUNT INFO REQUEST")
            
            response = {
                "makerCommission": 10,
                "takerCommission": 10,
                "buyerCommission": 0,
                "sellerCommission": 0,
                "canTrade": True,
                "canWithdraw": True,
                "canDeposit": True,
                "updateTime": int(time.time() * 1000),
                "accountType": "SPOT",
                "balances": [
                    {"asset": "BTC", "free": "1.50000000", "locked": "0.00000000"},
                    {"asset": "ETH", "free": "10.00000000", "locked": "0.00000000"},
                    {"asset": "USDT", "free": "50000.00000000", "locked": "0.00000000"},
                    {"asset": "SOL", "free": "500.00000000", "locked": "0.00000000"},
                    {"asset": "DOGE", "free": "100000.00000000", "locked": "0.00000000"},
                ],
                "permissions": ["SPOT"]
            }
            
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps(response).encode('utf-8'))

print(f"Starting Binance Mock Server on port {PORT}...")
with socketserver.TCPServer(("", PORT), BinanceMockHandler) as httpd:
    httpd.serve_forever()
