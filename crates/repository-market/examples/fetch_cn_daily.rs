use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use infra_parquet::ArchiveConfig;
use repository_market::MarketRepository;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::path::PathBuf;

fn parse_utc_day(input: &str) -> Result<DateTime<Utc>> {
    let date = NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .with_context(|| format!("invalid date '{}', expected format YYYY-MM-DD", input))?;
    let naive = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("invalid date value '{}'", input))?;
    Ok(DateTime::from_naive_utc_and_offset(naive, Utc))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        anyhow::bail!(
            "Usage: cargo run -p repository-market --example fetch_cn_daily -- <symbol> <start:YYYY-MM-DD> <end:YYYY-MM-DD> [provider=akshare] [exchange=SSE] [timeframe=1d]"
        );
    }

    let symbol = &args[1];
    let start = parse_utc_day(&args[2])?;
    let end = parse_utc_day(&args[3])?;
    let provider = args.get(4).map(String::as_str).unwrap_or("akshare");
    let exchange = args.get(5).map(String::as_str).unwrap_or("SSE");
    let timeframe = args.get(6).map(String::as_str).unwrap_or("1d");

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/postgres".to_string());
    let archive_root = env::var("ZQUANT_ARCHIVE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::temp_dir().join("zquant-archive"));
    std::fs::create_dir_all(&archive_root)
        .with_context(|| format!("failed to create archive root {}", archive_root.display()))?;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&database_url)
        .with_context(|| "failed to parse DATABASE_URL for lazy pool init")?;

    let repo = MarketRepository::new(pool, ArchiveConfig::new(archive_root));
    let bars = repo
        .load_bars_range(provider, exchange, symbol, timeframe, start, end)
        .await?;

    if bars.is_empty() {
        anyhow::bail!("拉取结果为空：请检查 provider/网络/Python akshare 环境");
    }

    println!(
        "拉取成功: provider={} symbol={} timeframe={} bars={}",
        provider,
        symbol,
        timeframe,
        bars.len()
    );
    for bar in bars.iter().take(5) {
        println!(
            "{} o={} h={} l={} c={} v={}",
            bar.timestamp.format("%Y-%m-%d"),
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume
        );
    }

    Ok(())
}
