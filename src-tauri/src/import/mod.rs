//! Tőzsdei és banki exportok beolvasása (Binance, Kraken, Revolut, Bybit, banki kivonat).
//!
//! Adózási szempontból csak a fiat ↔ kripto ügyletek számítanak: amikor fiatért kriptót veszel
//! (kiadás) vagy kriptót fiatra adsz el (bevétel). A díjak a kiadást növelik / a bevételt
//! csökkentik. A tőzsdére utalt vagy onnan kivett fiat, illetve a kripto–kripto ügyletek nem
//! számítanak, ezért ezeket kihagyjuk.
//!
//! Az ügyleteket naponta, devizánként és irányonként összevonjuk: az adó csak az éves
//! összegeken múlik, a napi MNB árfolyam így is pontos.

mod bank;
mod binance;
mod bybit;
mod kraken;
mod kraken_api;
mod revolut;

use calamine::{Data, Reader, open_workbook_auto};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Fiat devizák, amelyekre az MNB hivatalos árfolyamot közöl (és a forint).
const FIAT: &[&str] = &[
    "EUR", "USD", "HUF", "GBP", "CHF", "PLN", "CZK", "RON", "SEK", "NOK", "DKK", "JPY", "CAD", "AUD",
];

/// Egy ügylet fiat-oldala. `amount` < 0: fiat ment ki (vétel, díj), > 0: fiat jött be (eladás).
#[derive(Debug, Clone, PartialEq)]
pub struct FiatFlow {
    pub date: NaiveDate,
    pub currency: String,
    /// Előjeles nettó összeg, a díjjal együtt.
    pub amount: f64,
    /// Ebből a díj (tájékoztató, pozitív).
    pub fee: f64,
}

pub struct Parsed {
    pub source: &'static str,
    pub format: &'static str,
    pub flows: Vec<FiatFlow>,
    pub ignored: usize,
    pub warnings: Vec<String>,
}

/// Egy összevont napi tétel, ami az adatbázisba kerül.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportRow {
    /// Egyedi kulcs az újraimportáláshoz, pl. `binance:2026-03-04:EUR:buy`.
    pub key: String,
    pub date: String,
    pub currency: String,
    /// "buy" (fiat → kripto, kiadás) vagy "sell" (kripto → fiat, bevétel).
    pub kind: String,
    /// A fiat összeg abszolút értéke, díjjal együtt.
    pub amount: f64,
    pub fee: f64,
    /// Hány ügyletből áll össze.
    pub count: usize,
    /// Ha egy másik forrásból (pl. kézzel) már van hasonló tétel ugyanazon a napon, annak leírása.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate: Option<String>,
}

pub fn parse_file(path: &str) -> Result<Parsed, String> {
    let table = read_table(path)?;
    if binance::detect(&table) {
        binance::parse(&table)
    } else if kraken::detect(&table) {
        kraken::parse(&table)
    } else if revolut::detect(&table) {
        revolut::parse(&table)
    } else if bybit::detect(&table) {
        bybit::parse(&table)
    } else if bank::detect(&table) {
        // Utolsóként: a banki kivonat oszlopai általánosak, a tőzsdei formátumok elsőbbséget kapnak.
        bank::parse(&table)
    } else {
        Err(format!(
            "Nem ismerem fel a fájl formátumát. Talált oszlopok: {}",
            table.headers.iter().filter(|h| !h.is_empty()).cloned().collect::<Vec<_>>().join(", ")
        ))
    }
}

/// Kraken főkönyv lekérése API-n (csak olvasó kulccsal), majd a Ledgers exporttal azonos feldolgozás.
pub fn fetch_kraken(key: &str, secret: &str, start: i64) -> Result<Parsed, String> {
    let table = kraken_api::fetch_ledgers(key, secret, start)?;
    let mut parsed = kraken::parse(&table)?;
    parsed.format = "Kraken – API (főkönyv)";
    Ok(parsed)
}

pub fn aggregate(source: &str, flows: &[FiatFlow]) -> Vec<ImportRow> {
    let mut groups: BTreeMap<(NaiveDate, String, &'static str), (f64, f64, usize)> = BTreeMap::new();
    // A külön sorban szereplő díj (pl. Binance "Transaction Fee") ahhoz az aznapi eladáshoz
    // tartozik, ha van ilyen; különben a vételt növeli.
    let is_fee_only = |f: &FiatFlow| f.fee > 0.0 && (f.amount + f.fee).abs() < 1e-9;
    let sell_days: std::collections::BTreeSet<(NaiveDate, &str)> = flows
        .iter()
        .filter(|f| f.amount > 0.0)
        .map(|f| (f.date, f.currency.as_str()))
        .collect();
    for f in flows {
        if f.amount == 0.0 {
            continue;
        }
        let kind = if f.amount > 0.0 || (is_fee_only(f) && sell_days.contains(&(f.date, f.currency.as_str()))) {
            "sell"
        } else {
            "buy"
        };
        let g = groups.entry((f.date, f.currency.clone(), kind)).or_default();
        g.0 += f.amount;
        g.1 += f.fee;
        g.2 += 1;
    }
    groups
        .into_iter()
        .map(|((date, currency, kind), (amount, fee, count))| ImportRow {
            key: format!("{source}:{date}:{currency}:{kind}"),
            date: date.to_string(),
            currency,
            kind: kind.to_string(),
            amount: round2(amount.abs()),
            fee: round2(fee),
            count,
            duplicate: None,
        })
        .filter(|r| r.amount > 0.0)
        .collect()
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/* ───────────── Táblázat beolvasása ───────────── */

pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

impl Table {
    /// Az első oszlop, amelynek neve (kis/nagybetű, szóköz, írásjel nélkül) megegyezik valamelyik aliasszal.
    pub fn col(&self, aliases: &[&str]) -> Option<usize> {
        let heads: Vec<String> = self.headers.iter().map(|h| norm(h)).collect();
        aliases.iter().find_map(|a| {
            let a = norm(a);
            heads.iter().position(|h| *h == a)
        })
    }

    pub fn has(&self, aliases: &[&str]) -> bool {
        self.col(aliases).is_some()
    }

    pub fn require(&self, aliases: &[&str]) -> Result<usize, String> {
        self.col(aliases).ok_or_else(|| format!("Hiányzó oszlop: {}", aliases[0]))
    }
}

pub fn cell(row: &[String], i: usize) -> &str {
    row.get(i).map(|s| s.trim()).unwrap_or("")
}

pub fn read_table(path: &str) -> Result<Table, String> {
    let lower = path.to_lowercase();
    let rows = if lower.ends_with(".xlsx") || lower.ends_with(".xls") || lower.ends_with(".xlsm") {
        read_xlsx(path)?
    } else {
        let bytes = std::fs::read(path).map_err(|e| format!("Nem sikerült megnyitni: {e}"))?;
        read_csv(&String::from_utf8_lossy(&bytes))?
    };
    table_from_rows(rows)
}

/// A fejléc az első olyan sor, amelyben legalább 3 nem üres cella van (egyes exportok előtt van pár sor leírás).
fn table_from_rows(rows: Vec<Vec<String>>) -> Result<Table, String> {
    let start = rows
        .iter()
        .position(|r| r.iter().filter(|c| !c.trim().is_empty()).count() >= 3)
        .ok_or("A fájl üres vagy nem táblázat.")?;
    let mut it = rows.into_iter().skip(start);
    let headers = it.next().unwrap().into_iter().map(|h| h.trim().to_string()).collect();
    let rows = it.filter(|r| r.iter().any(|c| !c.trim().is_empty())).collect();
    Ok(Table { headers, rows })
}

pub fn read_csv(text: &str) -> Result<Vec<Vec<String>>, String> {
    let text = text.trim_start_matches('\u{feff}');
    let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let delim = [b',', b';', b'\t']
        .into_iter()
        .max_by_key(|d| first.matches(*d as char).count())
        .unwrap();
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delim)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    reader
        .records()
        .map(|r| r.map(|rec| rec.iter().map(str::to_string).collect()).map_err(|e| format!("Hibás CSV: {e}")))
        .collect()
}

fn read_xlsx(path: &str) -> Result<Vec<Vec<String>>, String> {
    let mut wb = open_workbook_auto(path).map_err(|e| format!("Nem sikerült megnyitni: {e}"))?;
    let first = wb.sheet_names().first().cloned().ok_or("Nincs munkalap a fájlban.")?;
    let range = wb.worksheet_range(&first).map_err(|e| e.to_string())?;
    Ok(range
        .rows()
        .map(|r| {
            r.iter()
                .map(|c| match c {
                    Data::Empty => String::new(),
                    Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
                    Data::Float(f) => f.to_string(),
                    Data::Int(i) => i.to_string(),
                    Data::Bool(b) => b.to_string(),
                    Data::DateTime(dt) => excel_serial_to_string(dt.as_f64()),
                    Data::Error(_) => String::new(),
                })
                .collect()
        })
        .collect())
}

fn excel_serial_to_string(serial: f64) -> String {
    let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let dt = base + Duration::milliseconds((serial * 86_400_000.0).round() as i64);
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/* ───────────── Értékek értelmezése ───────────── */

/// Fiat deviza kód normalizálása ("ZEUR", "EUR.HOLD", "eur" → "EUR"); kriptóra `None`.
pub fn fiat(code: &str) -> Option<String> {
    let mut c = code.trim().to_uppercase();
    if let Some(i) = c.find('.') {
        c.truncate(i);
    }
    if c.len() == 4 && c.starts_with('Z') && FIAT.contains(&&c[1..]) {
        c.remove(0);
    }
    FIAT.contains(&c.as_str()).then_some(c)
}

/// Pénzösszegben szereplő deviza ("€12.00", "USD 5", "1 000 Ft").
pub fn currency_in(s: &str) -> Option<String> {
    let s = s.trim();
    if s.contains('€') {
        return Some("EUR".into());
    }
    if s.contains('$') {
        return Some("USD".into());
    }
    if s.contains('£') {
        return Some("GBP".into());
    }
    if s.to_lowercase().ends_with("ft") {
        return Some("HUF".into());
    }
    s.split(|c: char| !c.is_ascii_alphabetic()).find_map(fiat)
}

/// Szám kiolvasása pénzformátumból: "1,234.56", "-€12.00", "1 234,56", "1.5E-7".
pub fn num(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(v) = s.parse::<f64>() {
        return Some(v);
    }
    let first_digit = s.find(|c: char| c.is_ascii_digit())?;
    let negative = s[..first_digit].contains(['-', '−']) || (s.starts_with('(') && s.ends_with(')'));
    let t: String = s[first_digit..].chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
    let t = match (t.rfind(','), t.rfind('.')) {
        (Some(c), Some(d)) if c > d => t.replace('.', "").replace(',', "."),
        (Some(_), Some(_)) => t.replace(',', ""),
        (Some(c), None) if t.len() - c - 1 == 3 && !t.starts_with("0,") => t.replace(',', ""),
        (Some(_), None) => t.replace(',', "."),
        (None, Some(_)) if t.matches('.').count() > 1 => t.replace('.', ""),
        _ => t,
    };
    let v: f64 = t.parse().ok()?;
    Some(if negative { -v } else { v })
}

/// Dátum-idő a gyakori exportformátumokból.
pub fn datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if s.len() >= 10 && s.chars().all(|c| c.is_ascii_digit()) {
        let n: i64 = s.parse().ok()?;
        let secs = if s.len() >= 13 { n / 1000 } else { n };
        return chrono::DateTime::from_timestamp(secs, 0).map(|d| d.naive_utc());
    }
    if let Ok(d) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(d.naive_utc());
    }
    let s = s.trim_end_matches('Z').trim_end_matches(" UTC").trim_end_matches("(UTC)").trim();
    const DT: &[&str] = &[
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d %H:%M",
        "%d/%m/%Y %H:%M:%S",
        "%d/%m/%Y %H:%M",
        "%d.%m.%Y %H:%M:%S",
        "%d.%m.%Y %H:%M",
        "%Y.%m.%d %H:%M:%S",
        "%Y. %m. %d. %H:%M:%S",
        "%b %d, %Y, %I:%M:%S %p",
        "%b %d, %Y %I:%M:%S %p",
        "%b %d, %Y, %H:%M:%S",
        "%d %b %Y, %H:%M:%S",
        "%d %b %Y %H:%M:%S",
    ];
    if let Some(d) = DT.iter().find_map(|f| NaiveDateTime::parse_from_str(s, f).ok()) {
        return Some(d);
    }
    const D: &[&str] = &["%Y-%m-%d", "%Y/%m/%d", "%d/%m/%Y", "%d.%m.%Y", "%Y.%m.%d", "%Y. %m. %d.", "%b %d, %Y", "%d %b %Y"];
    D.iter()
        .find_map(|f| NaiveDate::parse_from_str(s, f).ok())
        .map(|d| d.and_time(NaiveTime::MIN))
}

fn last_sunday(year: i32, month: u32) -> NaiveDate {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap();
    let mut d = next - Duration::days(1);
    while d.weekday() != Weekday::Sun {
        d -= Duration::days(1);
    }
    d
}

/// UTC időpont → magyarországi naptári nap (CET/CEST, EU nyári időszámítás).
pub fn budapest_date(utc: NaiveDateTime) -> NaiveDate {
    let y = utc.year();
    let start = last_sunday(y, 3).and_hms_opt(1, 0, 0).unwrap();
    let end = last_sunday(y, 10).and_hms_opt(1, 0, 0).unwrap();
    let offset = if utc >= start && utc < end { 2 } else { 1 };
    (utc + Duration::hours(offset)).date()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(num("1,234.56"), Some(1234.56));
        assert_eq!(num("-€12.00"), Some(-12.0));
        assert_eq!(num("€1,000"), Some(1000.0));
        assert_eq!(num("1 234,56"), Some(1234.56));
        assert_eq!(num("12,5"), Some(12.5));
        assert_eq!(num("-1.2E-7"), Some(-1.2e-7));
        assert_eq!(num("EUR 50.10"), Some(50.10));
        assert_eq!(num(""), None);
    }

    #[test]
    fn currencies() {
        assert_eq!(fiat("ZEUR").as_deref(), Some("EUR"));
        assert_eq!(fiat("EUR.HOLD").as_deref(), Some("EUR"));
        assert_eq!(fiat("BTC"), None);
        assert_eq!(fiat("USDT"), None);
        assert_eq!(currency_in("€1,000.00").as_deref(), Some("EUR"));
        assert_eq!(currency_in("USD 12").as_deref(), Some("USD"));
    }

    #[test]
    fn dates_and_timezone() {
        let d = datetime("2026-01-13 23:30:00").unwrap();
        assert_eq!(budapest_date(d), NaiveDate::from_ymd_opt(2026, 1, 14).unwrap());
        let s = datetime("2026-07-01T22:30:00Z").unwrap();
        assert_eq!(budapest_date(s), NaiveDate::from_ymd_opt(2026, 7, 2).unwrap());
        assert!(datetime("Jan 5, 2026, 10:23:45 AM").is_some());
        assert!(datetime("05/01/2026 10:23").is_some());
        assert!(datetime("1767225600000").is_some());
    }

    #[test]
    fn aggregates_per_day() {
        let d = NaiveDate::from_ymd_opt(2026, 3, 4).unwrap();
        let flows = vec![
            FiatFlow { date: d, currency: "EUR".into(), amount: -100.0, fee: 0.1 },
            FiatFlow { date: d, currency: "EUR".into(), amount: -50.0, fee: 0.05 },
            FiatFlow { date: d, currency: "EUR".into(), amount: 30.0, fee: 0.0 },
        ];
        let rows = aggregate("binance", &flows);
        assert_eq!(rows.len(), 2);
        let buy = rows.iter().find(|r| r.kind == "buy").unwrap();
        assert_eq!((buy.amount, buy.count, buy.key.as_str()), (150.0, 2, "binance:2026-03-04:EUR:buy"));
    }

    #[test]
    fn separate_fee_goes_to_same_day_sale() {
        let d = NaiveDate::from_ymd_opt(2026, 6, 3).unwrap();
        let flows = vec![
            FiatFlow { date: d, currency: "EUR".into(), amount: 196.8, fee: 0.0 },
            FiatFlow { date: d, currency: "EUR".into(), amount: -0.2, fee: 0.2 },
        ];
        let rows = aggregate("binance", &flows);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].kind.as_str(), rows[0].amount, rows[0].fee), ("sell", 196.6, 0.2));
    }

    /// Az `examples/` mappa mintafájljai mindig beolvashatók maradjanak (a README is ezekre hivatkozik).
    #[test]
    fn example_files_import() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
        let cases = [
            ("binance-transaction-history.csv", "binance", 4),
            ("kraken-ledgers.csv", "kraken", 2),
            ("revolut-crypto-statement.csv", "revolut", 4),
            ("revolut-crypto-statement-regi.csv", "revolut", 3),
            ("bybit-spot-trade-history.csv", "bybit", 3),
            ("bybit-convert-history.csv", "bybit", 2),
            ("bank-kivonat.csv", "bank", 3),
        ];
        for (file, source, expected) in cases {
            let p = parse_file(dir.join(file).to_str().unwrap()).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert_eq!(p.source, source, "{file}");
            let rows = aggregate(p.source, &p.flows);
            assert_eq!(rows.len(), expected, "{file}: {rows:#?}");
        }
        let excel = crate::excel::read_workbook(dir.join("sajat-excel-nyilvantartas.xlsx").to_str().unwrap()).unwrap();
        assert_eq!(excel.len(), 9);
        assert!(excel.iter().any(|(_, huf)| *huf == -250_000));
    }
}
