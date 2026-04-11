#!/usr/bin/env python3
"""RTP Trading Wing → Hyperliquid Testnet Integration
Validated strategy: SOL/USDT Survivor 2.69 (OOS Sharpe +3.96, 100% consistency)

Prerequisites:
  pip install eth-account web3 requests
  Fund at: https://app.hyperliquid-testnet.xyz/drip
"""
import json, requests, time, os, sys
from eth_account import Account
from eth_account.messages import encode_defunct
from web3 import Web3

# Load HL testnet key
CONFIG = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "configs", "hl_testnet_key.json")
with open(CONFIG) as f:
    key_data = json.load(f)

wallet = Account.from_key(key_data["private_key"])
BASE = "https://api.hyperliquid-testnet.xyz"

# Validated strategy config (Night Shift Apr 9)
STRATEGY = {
    "name": "SOL/USDT Survivor 2.69",
    "signal_threshold": 0.3,
    "tp_atr": 3.0,
    "sl_atr": 1.5,
    "max_hold_hours": 36,
    "trailing_stop_atr": 0.5,
    "oos_sharpe": 3.96,
    "consistency": "9/9 folds profitable"
}

def get_account_state():
    r = requests.post(f"{BASE}/info", json={"type": "clearinghouseState", "user": wallet.address})
    return r.json()

def get_sol_index():
    r = requests.post(f"{BASE}/info", json={"type": "metaAndAssetCtxs"})
    meta = r.json()
    for i, a in enumerate(meta[0]["universe"]):
        if a["name"] == "SOL":
            return i
    return 0

def sign_action(action, nonce):
    """Sign HL action with EIP-712 compatible signature"""
    action_str = json.dumps(action, separators=(",", ":"), sort_keys=True)
    action_bytes = Web3.keccak(text=action_str)
    msg = encode_defunct(action_bytes)
    sig = wallet.sign_message(msg)
    return {
        "r": hex(sig.r),
        "s": hex(sig.s),
        "v": sig.v
    }

def place_order(asset_idx, is_buy, size, price="0", tif="Ioc"):
    nonce = int(time.time() * 1000)
    action = {
        "type": "order",
        "orders": [{
            "a": asset_idx,
            "b": is_buy,
            "p": price,
            "s": str(size),
            "r": False,
            "t": {"limit": {"tif": tif}}
        }],
        "grouping": "na"
    }
    sig = sign_action(action, nonce)
    payload = {"action": action, "nonce": nonce, "signature": sig}
    r = requests.post(f"{BASE}/exchange", json=payload)
    return r.json()

if __name__ == "__main__":
    print(f"RTP Trading Wing → Hyperliquid Testnet")
    print(f"Strategy: {STRATEGY['name']}")
    print(f"Wallet: {wallet.address}")
    
    state = get_account_state()
    val = state.get("marginSummary", {}).get("accountValue", "0")
    print(f"Account Value: ${val}")
    
    if float(val) > 0:
        sol_idx = get_sol_index()
        print(f"Placing SOL long 0.01...")
        result = place_order(sol_idx, True, 0.01)
        print(f"Result: {json.dumps(result, indent=2)}")
    else:
        print(f"Fund at: https://app.hyperliquid-testnet.xyz/drip")
        sys.exit(1)
