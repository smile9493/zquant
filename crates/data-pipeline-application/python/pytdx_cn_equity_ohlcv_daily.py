"""
PyTDX Provider Plugin: CN equity daily OHLCV (SH/SZ/BJ)

Fetches historical daily OHLCV data from TDX servers for Chinese equity markets.
Supports Shanghai (SH), Shenzhen (SZ), and Beijing (BJ) exchanges.

Contract:
- Input (stdin JSON): {"symbol": "000001", "start_date": "20240101", "end_date": "20240301"}
- Output (stdout JSON): {"status": "success", "data": [{"date": "2024-01-02", "open": 1.0, ...}]}
- Error (stdout JSON): {"status": "error", "message": "..."}
"""

import json
import sys
from datetime import datetime
from typing import Any, Dict, List, Optional


def _fmt_yyyymmdd(s: str | None) -> str | None:
    """Convert date string to YYYYMMDD format."""
    if not s:
        return None
    s = s.strip()
    if len(s) == 8 and s.isdigit():
        return s
    try:
        return datetime.fromisoformat(s).strftime("%Y%m%d")
    except Exception:
        raise ValueError(f"invalid date format: {s!r}")


def _classify_market(symbol: str) -> tuple[int, str]:
    """
    Classify symbol into TDX market code.
    
    Returns: (market_code, exchange_label)
    - market_code: 0=SZ, 1=SH (for hq API)
    - exchange_label: "SH"/"SZ"/"BJ" for logging
    """
    if not symbol or len(symbol) < 6:
        raise ValueError(f"invalid symbol: {symbol!r}")
    
    prefix = symbol[:2]
    
    # Shanghai: 60xxxx (A-share), 50xxxx/51xxxx (funds), 688xxx (STAR)
    if prefix in ("60", "50", "51", "68"):
        return (1, "SH")
    
    # Shenzhen: 00xxxx (main), 30xxxx (ChiNext), 20xxxx (B-share)
    if prefix in ("00", "30", "20"):
        return (0, "SZ")
    
    # Beijing: 43xxxx, 83xxxx, 87xxxx, 88xxxx, 92xxxx (common prefixes)
    # Strategy: try hq market=0 first (some BJ may be in SZ market)
    if prefix in ("43", "83", "87", "88", "92"):
        return (0, "BJ")
    
    # Default to SZ for unknown prefixes (fail-closed will catch if wrong)
    return (0, "UNKNOWN")


def _connect_tdx():
    """
    Connect to TDX server with retry across multiple hosts.
    
    Returns: TdxHq_API instance (connected)
    Raises: RuntimeError if all hosts fail
    """
    from pytdx.hq import TdxHq_API  # type: ignore
    
    # Common public TDX servers (best-effort list)
    hosts = [
        ("119.147.212.81", 7709),
        ("114.80.63.12", 7709),
        ("180.153.18.170", 7709),
    ]
    
    api = TdxHq_API()
    for host, port in hosts:
        try:
            if api.connect(host, port):
                return api
        except Exception:
            continue
    
    raise RuntimeError("failed to connect to any TDX server")


def _fetch_bars_hq(api, market: int, symbol: str, start_date: Optional[str], end_date: Optional[str]) -> List[Dict[str, Any]]:
    """
    Fetch daily bars using hq API (category=9 for daily).
    
    TDX returns bars in reverse chronological order (newest first).
    We need to paginate with pos offset and reverse the final result.
    """
    category = 9  # daily
    count_per_page = 800  # TDX max
    
    all_bars = []
    pos = 0
    
    while True:
        bars = api.get_security_bars(category, market, symbol, pos, count_per_page)
        if not bars or len(bars) == 0:
            break
        
        all_bars.extend(bars)
        
        if len(bars) < count_per_page:
            break
        pos += count_per_page
    
    # Reverse to chronological order (oldest first)
    all_bars.reverse()
    
    # Filter by date range if specified
    if start_date or end_date:
        filtered = []
        for bar in all_bars:
            date_str = bar.get("datetime", "").replace("-", "")[:8]
            if start_date and date_str < start_date:
                continue
            if end_date and date_str > end_date:
                continue
            filtered.append(bar)
        all_bars = filtered
    
    return all_bars


def _try_fetch_bj_exhq(symbol: str, start_date: Optional[str], end_date: Optional[str]) -> List[Dict[str, Any]]:
    """
    Fallback: try fetching BJ symbol via exhq API (market="股份转让(SB)").
    
    This is a best-effort attempt — exhq may require instrument_code mapping.
    For now, we attempt direct symbol lookup.
    """
    from pytdx.exhq import TdxExHq_API  # type: ignore
    
    api = TdxExHq_API()
    # Use a known exhq server (best-effort)
    if not api.connect("106.14.95.149", 7727):
        raise RuntimeError("failed to connect to exhq server")
    
    try:
        # Attempt to get instrument info (may need code mapping)
        # For simplicity, we try direct symbol as instrument_code
        # This may fail — that's expected for unsupported BJ symbols
        markets = api.get_markets()
        sb_market = next((m for m in markets if "股份转让" in m.get("name", "") or "SB" in m.get("name", "")), None)
        if not sb_market:
            raise RuntimeError("exhq: 股份转让(SB) market not found")
        
        market_code = sb_market.get("market", 0)
        
        # Try fetching bars (this is speculative — may not work without proper instrument mapping)
        category = 7  # daily for exhq
        bars = api.get_instrument_bars(category, market_code, symbol, 0, 800)
        
        if not bars:
            raise RuntimeError(f"exhq returned no data for {symbol}")
        
        # Filter by date range
        if start_date or end_date:
            filtered = []
            for bar in bars:
                date_str = bar.get("datetime", "").replace("-", "")[:8]
                if start_date and date_str < start_date:
                    continue
                if end_date and date_str > end_date:
                    continue
                filtered.append(bar)
            bars = filtered
        
        return bars
    finally:
        api.disconnect()


def _normalize_bars(bars: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    """
    Normalize TDX bar dict to contract fields: date/open/high/low/close/volume.
    
    TDX returns: {"datetime": "2024-01-02", "open": 1.0, "high": 1.0, "low": 1.0, "close": 1.0, "vol": 123}
    Contract expects: {"date": "2024-01-02", "open": 1.0, "high": 1.0, "low": 1.0, "close": 1.0, "volume": 123.0}
    """
    records = []
    for bar in bars:
        # Extract date (TDX returns "YYYY-MM-DD" or "YYYY-MM-DD HH:MM")
        date_str = bar.get("datetime", "")
        if " " in date_str:
            date_str = date_str.split()[0]
        
        records.append({
            "date": date_str,
            "open": float(bar.get("open", 0.0)),
            "high": float(bar.get("high", 0.0)),
            "low": float(bar.get("low", 0.0)),
            "close": float(bar.get("close", 0.0)),
            "volume": float(bar.get("vol", 0.0)),
        })
    
    return records


def main() -> int:
    payload = json.load(sys.stdin)
    
    symbol = payload.get("symbol")
    if not symbol or not isinstance(symbol, str):
        raise ValueError("missing required field: symbol (string)")
    
    start_date = _fmt_yyyymmdd(payload.get("start_date"))
    end_date = _fmt_yyyymmdd(payload.get("end_date"))
    
    market_code, exchange = _classify_market(symbol)
    
    # Connect to TDX
    api = _connect_tdx()
    
    try:
        # Try hq first
        bars = _fetch_bars_hq(api, market_code, symbol, start_date, end_date)
        
        # If BJ and no data, try exhq fallback
        if exchange == "BJ" and len(bars) == 0:
            api.disconnect()
            bars = _try_fetch_bj_exhq(symbol, start_date, end_date)
        
        # Fail-closed: if still no data, return error (not empty success)
        if len(bars) == 0:
            raise RuntimeError(f"no data returned for {symbol} ({exchange})")
        
        records = _normalize_bars(bars)
        
        out = {"status": "success", "data": records}
        sys.stdout.write(json.dumps(out, ensure_ascii=False))
        return 0
    
    finally:
        if api:
            api.disconnect()


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as e:
        error_out = {"status": "error", "message": str(e)}
        sys.stdout.write(json.dumps(error_out, ensure_ascii=False))
        sys.exit(1)
