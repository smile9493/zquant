import json
import sys
from datetime import datetime


def _fmt_yyyymmdd(s: str | None) -> str | None:
    if not s:
        return None
    # Accept YYYY-MM-DD or YYYYMMDD
    s = s.strip()
    if len(s) == 8 and s.isdigit():
        return s
    try:
        return datetime.fromisoformat(s).strftime("%Y%m%d")
    except Exception:
        raise ValueError(f"invalid date format: {s!r}")


def main() -> int:
    payload = json.load(sys.stdin)

    symbol = payload.get("symbol")
    if not symbol or not isinstance(symbol, str):
        raise ValueError("missing required field: symbol (string)")

    start_date = _fmt_yyyymmdd(payload.get("start_date"))
    end_date = _fmt_yyyymmdd(payload.get("end_date"))
    adjust = payload.get("adjust") or ""

    import akshare as ak  # type: ignore

    # AkShare returns a pandas DataFrame. We emit JSON records for Rust to parse.
    df = ak.stock_zh_a_hist(
        symbol=symbol,
        period="daily",
        start_date=start_date,
        end_date=end_date,
        adjust=adjust,
    )

    # Normalize column names to a minimal OHLCV shape.
    # We keep extra fields if present; Rust-side normalizer/DQ can choose what to use.
    records = df.to_dict(orient="records")

    out = {"status": "success", "data": records}
    sys.stdout.write(json.dumps(out, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as e:
        error_out = {"status": "error", "message": str(e)}
        sys.stdout.write(json.dumps(error_out, ensure_ascii=False))
        sys.exit(1)

