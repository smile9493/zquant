#!/usr/bin/env python3
"""Test script to verify Chinese column name mapping without requiring akshare."""
import json
import sys


def main() -> int:
    # Read input (not used, but maintains contract)
    _ = json.load(sys.stdin)

    # Simulate AkShare output with Chinese column names
    mock_data = [
        {
            "日期": "2024-01-01",
            "开盘": 10.5,
            "收盘": 10.8,
            "最高": 11.0,
            "最低": 10.2,
            "成交量": 1000000,
        }
    ]

    # Apply the same mapping as the real script
    column_mapping = {
        "日期": "date",
        "开盘": "open",
        "收盘": "close",
        "最高": "high",
        "最低": "low",
        "成交量": "volume",
    }

    mapped_data = []
    for record in mock_data:
        mapped_record = {column_mapping.get(k, k): v for k, v in record.items()}
        mapped_data.append(mapped_record)

    out = {"status": "success", "data": mapped_data}
    sys.stdout.write(json.dumps(out, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as e:
        error_out = {"status": "error", "message": str(e)}
        sys.stdout.write(json.dumps(error_out, ensure_ascii=False))
        sys.exit(1)
