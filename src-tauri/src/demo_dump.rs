//! Bemutató adatok a weboldal képernyőképeihez (`brand/app-screens.json`), a valódi adószámítással.
//! Újragenerálás: `cargo test --manifest-path src-tauri/Cargo.toml dump_demo -- --ignored`
//! (a képek elkészítése: `brand/app-screens.html`).
use super::*;

/// (dátum, deviza, összeg, árfolyam, forrás, típus, díj, ügyletszám, megjegyzés)
type DemoRow<'a> = (&'a str, &'a str, f64, f64, &'a str, &'a str, f64, i64, &'a str);

#[test]
#[ignore]
fn dump_demo() {
    let conn = Connection::open_in_memory().unwrap();
    init_db(&conn).unwrap();
    let rows: &[DemoRow] = &[
        ("2023-02-14", "EUR", 1000.0, 386.52, "binance", "buy", 1.0, 2, ""),
        ("2023-05-09", "EUR", 500.0, 372.10, "revolut", "buy", 7.5, 1, ""),
        ("2023-11-20", "EUR", -600.0, 381.24, "binance", "sell", 0.6, 1, ""),
        ("2024-01-15", "EUR", 800.0, 380.63, "kraken", "buy", 2.08, 1, ""),
        ("2024-03-11", "EUR", -600.0, 393.41, "kraken", "sell", 1.56, 1, ""),
        ("2024-06-05", "EUR", 300.0, 384.92, "revolut", "buy", 4.5, 1, ""),
        ("2025-02-03", "EUR", 1200.0, 410.95, "binance", "buy", 1.2, 3, ""),
        ("2025-04-22", "EUR", -1800.0, 404.33, "bybit", "sell", 1.8, 2, ""),
        ("2025-08-18", "HUF", 150000.0, 1.0, "kézi", "deposit", 0.0, 1, "Revolut feltöltés"),
        ("2025-10-07", "EUR", -400.0, 389.80, "cryptocom", "payment", 0.0, 1, ""),
        ("2026-01-12", "EUR", 1000.0, 394.20, "binance", "buy", 1.0, 2, ""),
        ("2026-03-04", "EUR", 400.0, 390.05, "kraken", "buy", 1.04, 1, ""),
        ("2026-05-19", "EUR", -2500.0, 384.51, "binance", "sell", 2.5, 2, ""),
        ("2026-06-30", "EUR", 250.0, 383.10, "kézi", "income", 0.0, 1, "Grafikai munka díja USDC-ben"),
        ("2026-07-08", "EUR", -150.0, 382.33, "cryptocom", "payment", 0.0, 1, ""),
        ("2026-09-15", "EUR", 600.0, 387.62, "revolut", "buy", 9.0, 1, ""),
    ];
    for (i, (date, cur, amount, rate, source, kind, fee, trades, note)) in rows.iter().enumerate() {
        let huf = (amount * rate).round() as i64;
        let (rs, ext) = if *source == "kézi" { (if *cur == "HUF" { "-" } else { "MNB" }, None) } else { ("MNB", Some(format!("{source}:{date}:{cur}:{kind}"))) };
        let label = match *kind { "buy" => "vétel", "payment" => "kártyás fizetés", _ => "eladás" };
        let note = if note.is_empty() { format!("{} · {trades} {label}", source_label(source)) } else { note.to_string() };
        conn.execute(
            "INSERT INTO transactions (date, currency, amount, rate, rate_source, rate_date, huf, note, source, kind, fee, external_id, trades)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![date, cur, amount, rate, rs, date, huf, note, source, kind, fee, ext, trades],
        )
        .unwrap();
        if let Some(ext) = ext {
            let origin = match *source {
                "binance" => "Binance export: binance-2026.csv · SHA-256 3f9a…c21e",
                "kraken" => "Kraken API · lekérve 2026-11-02 10:14 · 2024-01-01 óta",
                "revolut" => "Revolut kivonat: revolut-crypto.xlsx · SHA-256 81d0…5b7a",
                "bybit" => "Bybit export: bybit-convert.csv · SHA-256 c44e…09f2",
                _ => "Crypto.com export: crypto_transactions_record.csv · SHA-256 aa12…e771",
            };
            for t in 0..*trades {
                let (h, part) = (9 + i % 8, amount / *trades as f64);
                let raw = match *source {
                    "binance" => format!("41827365;{date} {h:02}:1{t}:3{t};Spot;{};{cur};{part:.2};", if *kind == "buy" { "Transaction Spend" } else { "Transaction Revenue" }),
                    "kraken" => format!("L4XQ2Z-7KD3F-{i:02}{t}AQ;T5RB2K-{t}{i:02}9C;{date} {h:02}:0{t}:12;trade;;currency;ZEUR;{part:.4};{:.4}", fee / *trades as f64),
                    "revolut" => format!("{date} {h:02}:4{t}:05;Exchange;BTC;{cur};{part:.2};Completed"),
                    "bybit" => format!("{date} {h:02}:2{t}:44;BTCEUR;Sell;Filled;{part:.2};{cur}"),
                    _ => format!("{date}T{h:02}:3{t}:00Z;Card Top-Up;BTC;{cur};{part:.2};card_top_up"),
                };
                conn.execute("INSERT INTO import_details (external_id, origin, raw) VALUES (?1, ?2, ?3)", params![ext, origin, raw]).unwrap();
            }
        }
    }
    conn.execute("INSERT INTO loss_declarations (year, declared) VALUES (2023, 1), (2024, 1)", []).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
    let details: Vec<(i64, ImportDetail)> = import_details(&conn, None).unwrap();
    let out = serde_json::json!({
        "summary": compute_summary(&conn).unwrap(),
        "transactions": all_transactions(&conn, None).unwrap(),
        "issues": checks::issues(&conn, today).unwrap(),
        "plan": checks::plan(&conn, 2026, 550_425, today).unwrap(),
        "details": details.iter().map(|(id, d)| serde_json::json!({"id": id, "origin": d.origin, "raw": d.raw})).collect::<Vec<_>>(),
    });
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/../brand/app-screens.json"), serde_json::to_string_pretty(&out).unwrap()).unwrap();
}
