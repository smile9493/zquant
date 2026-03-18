#!/usr/bin/env python3
"""
Hermetic test for pytdx_cn_equity_ohlcv_daily.py contract.

Validates:
- Market classification logic (SH/SZ/BJ/unknown)
- Date format normalization
- Bar normalization (TDX fields -> contract fields)
- Date string serialization (no datetime objects in output)
- Fail-closed on empty data

Does NOT import pytdx — all TDX responses are mocked.
"""

import json
import sys
import os

# Add parent directory so we can import the module's functions
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# We import only the pure functions, not main() which needs pytdx
from pytdx_cn_equity_ohlcv_daily import _classify_market, _fmt_yyyymmdd, _normalize_bars


def test_classify_market_sh():
    """Shanghai symbols: 60xxxx, 50xxxx, 51xxxx, 68xxxx"""
    assert _classify_market("600000") == (1, "SH"), "600000 should be SH"
    assert _classify_market("688001") == (1, "SH"), "688001 (STAR) should be SH"
    assert _classify_market("510050") == (1, "SH"), "510050 (ETF) should be SH"
    assert _classify_market("500001") == (1, "SH"), "500001 should be SH"


def test_classify_market_sz():
    """Shenzhen symbols: 00xxxx, 30xxxx, 20xxxx"""
    assert _classify_market("000001") == (0, "SZ"), "000001 should be SZ"
    assert _classify_market("300001") == (0, "SZ"), "300001 (ChiNext) should be SZ"
    assert _classify_market("200001") == (0, "SZ"), "200001 (B-share) should be SZ"


def test_classify_market_bj():
    """Beijing symbols: 43xxxx, 83xxxx, 87xxxx, 88xxxx, 92xxxx"""
    for prefix in ("43", "83", "87", "88", "92"):
        sym = prefix + "0001"
        code, label = _classify_market(sym)
        assert label == "BJ", f"{sym} should be BJ, got {label}"
        assert code == 0, f"{sym} market code should be 0 (try SZ first)"


def test_classify_market_invalid():
    """Invalid symbols should raise ValueError"""
    try:
        _classify_market("")
        assert False, "empty symbol should raise"
    except ValueError:
        pass

    try:
        _classify_market("123")
        assert False, "short symbol should raise"
    except ValueError:
        pass


def test_fmt_yyyymmdd():
    """Date format normalization"""
    assert _fmt_yyyymmdd("20240101") == "20240101"
    assert _fmt_yyyymmdd("2024-01-01") == "20240101"
    assert _fmt_yyyymmdd(None) is None
    assert _fmt_yyyymmdd("") is None

    try:
        _fmt_yyyymmdd("not-a-date")
        assert False, "invalid date should raise"
    except ValueError:
        pass


def test_normalize_bars():
    """TDX bar dict -> contract fields"""
    mock_bars = [
        {
            "datetime": "2024-01-02 15:00",
            "open": 9.50,
            "high": 9.80,
            "low": 9.40,
            "close": 9.70,
            "vol": 500000,
        },
        {
            "datetime": "2024-01-03",
            "open": 9.70,
            "high": 10.00,
            "low": 9.60,
            "close": 9.90,
            "vol": 600000,
        },
    ]

    records = _normalize_bars(mock_bars)
    assert len(records) == 2

    r0 = records[0]
    assert r0["date"] == "2024-01-02", f"date should strip time: {r0['date']}"
    assert r0["open"] == 9.50
    assert r0["high"] == 9.80
    assert r0["low"] == 9.40
    assert r0["close"] == 9.70
    assert r0["volume"] == 500000.0

    r1 = records[1]
    assert r1["date"] == "2024-01-03"


def test_normalize_bars_date_serializable():
    """Ensure all date fields are strings (no datetime objects)"""
    mock_bars = [
        {"datetime": "2024-06-15", "open": 1.0, "high": 1.0, "low": 1.0, "close": 1.0, "vol": 100},
    ]
    records = _normalize_bars(mock_bars)

    # Verify JSON serialization works (would fail if date is datetime object)
    json_str = json.dumps(records)
    parsed = json.loads(json_str)
    assert isinstance(parsed[0]["date"], str), "date must be a string after JSON round-trip"


def test_normalize_bars_empty():
    """Empty input produces empty output"""
    assert _normalize_bars([]) == []


def test_output_contract_shape():
    """Verify the full output JSON matches the contract schema"""
    mock_bars = [
        {"datetime": "2024-01-02", "open": 10.0, "high": 11.0, "low": 9.5, "close": 10.5, "vol": 1000},
    ]
    records = _normalize_bars(mock_bars)
    output = {"status": "success", "data": records}

    # Serialize and re-parse to verify JSON contract
    raw = json.dumps(output, ensure_ascii=False)
    parsed = json.loads(raw)

    assert parsed["status"] == "success"
    assert isinstance(parsed["data"], list)
    assert len(parsed["data"]) == 1

    rec = parsed["data"][0]
    required_fields = {"date", "open", "high", "low", "close", "volume"}
    assert required_fields.issubset(set(rec.keys())), f"missing fields: {required_fields - set(rec.keys())}"


def main():
    tests = [
        test_classify_market_sh,
        test_classify_market_sz,
        test_classify_market_bj,
        test_classify_market_invalid,
        test_fmt_yyyymmdd,
        test_normalize_bars,
        test_normalize_bars_date_serializable,
        test_normalize_bars_empty,
        test_output_contract_shape,
    ]

    passed = 0
    failed = 0
    for t in tests:
        try:
            t()
            passed += 1
            print(f"  PASS: {t.__name__}")
        except Exception as e:
            failed += 1
            print(f"  FAIL: {t.__name__}: {e}")

    print(f"\n{passed} passed, {failed} failed, {passed + failed} total")
    return 1 if failed > 0 else 0


if __name__ == "__main__":
    raise SystemExit(main())
