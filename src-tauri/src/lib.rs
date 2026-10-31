mod backup;
mod data;
mod excel;
mod import;
mod mnb;
mod pdf;
mod tax;

use chrono::{Datelike, Duration, Local, NaiveDate};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

type Db = Arc<Mutex<Connection>>;

struct AppState {
    /// Az aktív profil adatbázisa; profilváltáskor a kapcsolat cserélődik.
    db: Db,
    /// Az app adatkönyvtára (adatbázisok, mentések, profilok).
    dir: PathBuf,
    profile: Mutex<String>,
}

/// Az alapértelmezett profil azonosítója; adatbázisa a régi helyen marad (`kripto-ado.sqlite`).
const DEFAULT_PROFILE: &str = "sajat";

fn db_path(dir: &Path, profile: &str) -> PathBuf {
    if profile == DEFAULT_PROFILE {
        dir.join("kripto-ado.sqlite")
    } else {
        dir.join("profiles").join(format!("{profile}.sqlite"))
    }
}

fn open_db(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(err)?;
    }
    let conn = Connection::open(path).map_err(err)?;
    init_db(&conn).map_err(err)?;
    Ok(conn)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Tx {
    id: i64,
    date: String,
    currency: String,
    /// Pozitív: befizetés, negatív: kivétel.
    amount: f64,
    rate: f64,
    /// "MNB", "kézi" vagy "-" (forintos tételnél).
    rate_source: String,
    /// Az MNB árfolyam közzétételének napja (hétvégén/ünnepnap az előző munkanap).
    rate_date: Option<String>,
    huf: i64,
    note: String,
    /// "kézi", "excel", "binance", "kraken", "revolut", "bybit".
    source: String,
    /// "deposit" / "withdrawal" (kézi), "buy" / "sell" (tőzsdei import).
    kind: String,
    /// Ebből a díj a tétel devizájában (tájékoztató).
    fee: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxInput {
    date: String,
    currency: String,
    amount: f64,
    /// Ha üres, az MNB árfolyamot használjuk.
    rate: Option<f64>,
    note: Option<String>,
    /// "payment": kriptóval fizetett áru vagy szolgáltatás (bevételnek számít, mint a kivétel).
    #[serde(default)]
    kind: Option<String>,
}

impl TxInput {
    fn kind(&self) -> &'static str {
        if self.amount < 0.0 && self.kind.as_deref() == Some("payment") { "payment" } else { manual_kind(self.amount) }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RateInfo {
    rate: f64,
    rate_date: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    imported: usize,
    skipped: usize,
}

fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS transactions (
            id          INTEGER PRIMARY KEY,
            date        TEXT NOT NULL,
            currency    TEXT NOT NULL,
            amount      REAL NOT NULL,
            rate        REAL NOT NULL,
            rate_source TEXT NOT NULL,
            rate_date   TEXT,
            huf         INTEGER NOT NULL,
            note        TEXT NOT NULL DEFAULT '',
            created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS transactions_date ON transactions(date);
        CREATE TABLE IF NOT EXISTS rates (
            currency TEXT NOT NULL,
            date     TEXT NOT NULL,
            rate     REAL NOT NULL,
            PRIMARY KEY (currency, date)
        );
        CREATE TABLE IF NOT EXISTS loss_declarations (
            year     INTEGER PRIMARY KEY,
            declared INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )?;
    let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(
            "ALTER TABLE transactions ADD COLUMN source TEXT NOT NULL DEFAULT 'kézi';
            ALTER TABLE transactions ADD COLUMN kind TEXT NOT NULL DEFAULT '';
            ALTER TABLE transactions ADD COLUMN fee REAL NOT NULL DEFAULT 0;
            ALTER TABLE transactions ADD COLUMN external_id TEXT;
            CREATE UNIQUE INDEX IF NOT EXISTS transactions_external ON transactions(external_id);
            UPDATE transactions SET source = 'excel' WHERE note = 'Excel import';
            UPDATE transactions SET kind = CASE WHEN amount > 0 THEN 'deposit' ELSE 'withdrawal' END WHERE kind = '';
            PRAGMA user_version = 1;",
        )?;
    }
    if version < 2 {
        // Importált napi soroknál az összevont ügyletek száma (a kisértékű ügylet szabályához).
        conn.execute_batch(
            "ALTER TABLE transactions ADD COLUMN trades INTEGER NOT NULL DEFAULT 1;
            PRAGMA user_version = 2;",
        )?;
    }
    Ok(())
}

fn manual_kind(amount: f64) -> &'static str {
    if amount > 0.0 { "deposit" } else { "withdrawal" }
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map_err(|_| format!("Érvénytelen dátum: {s}"))
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Az adott napon érvényes MNB árfolyam: a napon vagy azt megelőzően utoljára közzétett érték.
async fn mnb_rate(db: &Db, currency: &str, date: NaiveDate) -> Result<RateInfo, String> {
    let key = date.to_string();
    {
        let conn = db.lock().unwrap();
        let cached: Option<f64> = conn
            .query_row("SELECT rate FROM rates WHERE currency = ?1 AND date = ?2", params![currency, key], |r| r.get(0))
            .optional()
            .map_err(err)?;
        if let Some(rate) = cached {
            return Ok(RateInfo { rate, rate_date: key });
        }
    }

    let cur = currency.to_string();
    let days = tauri::async_runtime::spawn_blocking(move || mnb::fetch_range(&cur, date - Duration::days(14), date))
        .await
        .map_err(err)??;

    let conn = db.lock().unwrap();
    for (d, rate) in &days {
        conn.execute(
            "INSERT OR REPLACE INTO rates (currency, date, rate) VALUES (?1, ?2, ?3)",
            params![currency, d.to_string(), rate],
        )
        .map_err(err)?;
    }
    days.iter()
        .filter(|(d, _)| *d <= date)
        .max_by_key(|(d, _)| *d)
        .map(|(d, rate)| RateInfo { rate: *rate, rate_date: d.to_string() })
        .ok_or_else(|| format!("Nincs MNB {currency} árfolyam {date} előtti 14 napban."))
}

/// Ellenőrzi a bemenetet és kiszámolja az árfolyamot és a forintösszeget.
async fn resolve(db: &Db, input: &TxInput) -> Result<(NaiveDate, String, f64, String, Option<String>, i64), String> {
    let date = parse_date(&input.date)?;
    let currency = input.currency.trim().to_uppercase();
    if !input.amount.is_finite() || input.amount == 0.0 {
        return Err("Az összeg nem lehet nulla.".into());
    }
    let (rate, source, rate_date) = match (currency.as_str(), input.rate) {
        ("HUF", _) => (1.0, "-".to_string(), None),
        (_, Some(r)) if r > 0.0 => (r, "kézi".to_string(), None),
        (_, Some(_)) => return Err("Az árfolyamnak pozitívnak kell lennie.".into()),
        (_, None) => {
            let info = mnb_rate(db, &currency, date).await?;
            (info.rate, "MNB".to_string(), Some(info.rate_date))
        }
    };
    let huf = (input.amount * rate).round() as i64;
    Ok((date, currency, rate, source, rate_date, huf))
}

fn row_to_tx(r: &rusqlite::Row) -> rusqlite::Result<Tx> {
    Ok(Tx {
        id: r.get(0)?,
        date: r.get(1)?,
        currency: r.get(2)?,
        amount: r.get(3)?,
        rate: r.get(4)?,
        rate_source: r.get(5)?,
        rate_date: r.get(6)?,
        huf: r.get(7)?,
        note: r.get(8)?,
        source: r.get(9)?,
        kind: r.get(10)?,
        fee: r.get(11)?,
    })
}

const TX_COLUMNS: &str = "id, date, currency, amount, rate, rate_source, rate_date, huf, note, source, kind, fee";

fn load_tx(conn: &Connection, id: i64) -> Result<Tx, String> {
    conn.query_row(&format!("SELECT {TX_COLUMNS} FROM transactions WHERE id = ?1"), [id], row_to_tx)
        .map_err(err)
}

fn all_transactions(conn: &Connection, year: Option<i32>) -> Result<Vec<Tx>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {TX_COLUMNS} FROM transactions WHERE ?1 IS NULL OR substr(date, 1, 4) = ?1 ORDER BY date DESC, id DESC"
        ))
        .map_err(err)?;
    let rows = stmt
        .query_map([year.map(|y| y.to_string())], row_to_tx)
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    Ok(rows)
}

fn compute_summary(conn: &Connection) -> Result<Vec<tax::YearSummary>, String> {
    compute_summary_with(conn, None)
}

fn compute_summary_with(conn: &Connection, extra: Option<Hypothetical>) -> Result<Vec<tax::YearSummary>, String> {
    compute_summary_inner(conn, extra, |_| {})
}

/// `adjust_undeclared`: a bevallatlan évek halmazának módosítása (pl. „mi lenne, ha bevallanám”).
fn compute_summary_inner(
    conn: &Connection,
    extra: Option<Hypothetical>,
    adjust_undeclared: impl FnOnce(&mut BTreeSet<i32>),
) -> Result<Vec<tax::YearSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT CAST(substr(date, 1, 4) AS INTEGER),
                    COALESCE(SUM(CASE WHEN huf > 0 THEN huf END), 0),
                    COALESCE(SUM(CASE WHEN huf < 0 THEN -huf END), 0)
             FROM transactions WHERE id IS NOT ?1 GROUP BY 1",
        )
        .map_err(err)?;
    let mut totals: BTreeMap<i32, (i64, i64)> = stmt
        .query_map([extra.and_then(|e| e.exclude_id)], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))
        .map_err(err)?
        .collect::<Result<_, _>>()
        .map_err(err)?;
    if let Some(e) = extra {
        let t = totals.entry(e.year).or_default();
        if e.huf > 0 { t.0 += e.huf } else { t.1 -= e.huf }
    }
    // Kisértékű ügyletek (67/C. § (2)): bekapcsolva a mentesíthető bevétel kimarad az eredményből.
    let small = small_value_exempt(conn, extra.and_then(|e| e.exclude_id))?;
    let apply_small = setting(conn, "kiserteku")?.as_deref() == Some("on");
    if apply_small {
        for (y, exempt) in &small {
            if let Some(t) = totals.get_mut(y) {
                t.1 -= exempt;
            }
        }
    }
    let today = Local::now().date_naive();
    let mut undeclared = undeclared_years(conn, &totals, today)?;
    adjust_undeclared(&mut undeclared);
    let mut out = tax::summarize(&totals, today.year(), &undeclared);
    for s in &mut out {
        s.small_exempt_available = small.get(&s.year).copied().unwrap_or(0);
        if apply_small {
            s.small_exempt_huf = s.small_exempt_available;
            // A kimutatott kivétel a teljes összeg marad; az eredményből hiányzik a mentes rész.
            s.withdrawals_huf += s.small_exempt_huf;
        }
    }
    Ok(out)
}

fn setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional().map_err(err)
}

/// Évente a kisértékű ügyletként mentesíthető bevétel. Csak az a nap számít, amelyen egyetlen
/// bevételt hozó ügylet volt (importnál a napi összevont sor ügyletszáma alapján).
fn small_value_exempt(conn: &Connection, exclude_id: Option<i64>) -> Result<BTreeMap<i32, i64>, String> {
    let days: Vec<tax::SaleDay> = conn
        .prepare(
            "SELECT CAST(substr(date, 1, 4) AS INTEGER), SUM(trades), SUM(-huf)
             FROM transactions WHERE huf < 0 AND id IS NOT ?1 GROUP BY date",
        )
        .and_then(|mut s| {
            s.query_map([exclude_id], |r| Ok(tax::SaleDay { year: r.get(0)?, trades: r.get(1)?, huf: r.get(2)? }))?
                .collect()
        })
        .map_err(err)?;
    Ok(tax::small_value_exempt(&days))
}

/// Egy bevallatlan veszteséges év pótlásának hatása.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationImpact {
    year: i32,
    loss_huf: i64,
    /// Ennyivel kevesebb adó (vagy több visszajáró) a későbbi években, ha a veszteséget bevallják.
    saving: i64,
}

/// Önellenőrzés-segéd: a bevallatlan veszteséges évek, és mennyit érne a pótlásuk.
#[tauri::command]
fn declaration_impact(state: State<'_, AppState>) -> Result<Vec<DeclarationImpact>, String> {
    let conn = state.db.lock().unwrap();
    let base = compute_summary(&conn)?;
    let total_after = |s: &[tax::YearSummary], y: i32| s.iter().filter(|x| x.year > y).map(|x| x.tax_due).sum::<i64>();
    let mut out = Vec::new();
    for y in base.iter().filter(|s| s.loss_credit > 0 && !s.loss_declared) {
        let alt = compute_summary_inner(&conn, None, |u| {
            u.remove(&y.year);
        })?;
        out.push(DeclarationImpact {
            year: y.year,
            loss_huf: -y.result_huf,
            saving: total_after(&base, y.year) - total_after(&alt, y.year),
        });
    }
    Ok(out)
}

/// Azok az évek, amelyek veszteségét nem vallották be. Ha a felhasználó nem jelölte, a már lejárt
/// határidejű évet (május 20.) bevallatlannak vesszük, a még be nem adottat bevallottnak.
fn undeclared_years(
    conn: &Connection,
    totals: &BTreeMap<i32, (i64, i64)>,
    today: NaiveDate,
) -> Result<BTreeSet<i32>, String> {
    let marked: BTreeMap<i32, bool> = conn
        .prepare("SELECT year, declared FROM loss_declarations")
        .and_then(|mut s| s.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect())
        .map_err(err)?;
    let Some(&first) = totals.keys().next() else { return Ok(BTreeSet::new()) };
    Ok((first.max(tax::FIRST_YEAR)..=today.year())
        .filter(|y| {
            let deadline_passed = NaiveDate::from_ymd_opt(y + 1, 5, 20).is_some_and(|d| today > d);
            !marked.get(y).copied().unwrap_or(!deadline_passed)
        })
        .collect())
}

/// Bevallotta-e a felhasználó az adott év veszteségét.
#[tauri::command]
fn set_loss_declared(state: State<'_, AppState>, year: i32, declared: bool) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .execute("INSERT OR REPLACE INTO loss_declarations (year, declared) VALUES (?1, ?2)", params![year, declared])
        .map_err(err)?;
    Ok(())
}

/// Egy még nem mentett (vagy éppen szerkesztett) tétel hatása az adóra.
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
struct Hypothetical {
    year: i32,
    huf: i64,
    /// Szerkesztésnél a régi változat kihagyása.
    exclude_id: Option<i64>,
}

/// Az adott év összesítője úgy, mintha a tétel már rögzítve lenne.
#[tauri::command]
fn preview_tax(state: State<'_, AppState>, tx: Hypothetical) -> Result<Option<tax::YearSummary>, String> {
    let years = compute_summary_with(&state.db.lock().unwrap(), Some(tx))?;
    Ok(years.into_iter().find(|y| y.year == tx.year))
}

#[tauri::command]
fn list_transactions(state: State<'_, AppState>, year: Option<i32>) -> Result<Vec<Tx>, String> {
    all_transactions(&state.db.lock().unwrap(), year)
}

#[tauri::command]
async fn add_transaction(state: State<'_, AppState>, input: TxInput) -> Result<Tx, String> {
    let (date, currency, rate, source, rate_date, huf) = resolve(&state.db, &input).await?;
    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO transactions (date, currency, amount, rate, rate_source, rate_date, huf, note, source, kind)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'kézi', ?9)",
        params![date.to_string(), currency, input.amount, rate, source, rate_date, huf, input.note.as_deref().unwrap_or("").trim(), input.kind()],
    )
    .map_err(err)?;
    load_tx(&conn, conn.last_insert_rowid())
}

#[tauri::command]
async fn update_transaction(state: State<'_, AppState>, id: i64, input: TxInput) -> Result<Tx, String> {
    let (date, currency, rate, source, rate_date, huf) = resolve(&state.db, &input).await?;
    let conn = state.db.lock().unwrap();
    conn.execute(
        "UPDATE transactions SET date = ?1, currency = ?2, amount = ?3, rate = ?4, rate_source = ?5,
                rate_date = ?6, huf = ?7, note = ?8,
                kind = CASE WHEN kind IN ('buy', 'sell') THEN (CASE WHEN ?3 > 0 THEN 'buy' ELSE 'sell' END) ELSE ?10 END
         WHERE id = ?9",
        params![date.to_string(), currency, input.amount, rate, source, rate_date, huf, input.note.as_deref().unwrap_or("").trim(), id, input.kind()],
    )
    .map_err(err)?;
    load_tx(&conn, id)
}

#[tauri::command]
fn delete_transaction(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.db.lock().unwrap().execute("DELETE FROM transactions WHERE id = ?1", [id]).map_err(err)?;
    Ok(())
}

#[tauri::command]
async fn get_mnb_rate(state: State<'_, AppState>, date: String, currency: Option<String>) -> Result<RateInfo, String> {
    let currency = currency.unwrap_or_else(|| "EUR".into()).to_uppercase();
    mnb_rate(&state.db, &currency, parse_date(&date)?).await
}

#[tauri::command]
fn summary(state: State<'_, AppState>) -> Result<Vec<tax::YearSummary>, String> {
    compute_summary(&state.db.lock().unwrap())
}

/// A régi Excel munkafüzet importja: évszámmal elnevezett lapok, A oszlop dátum, B oszlop forintösszeg.
#[tauri::command(async)]
fn import_excel(state: State<'_, AppState>, path: String) -> Result<ImportResult, String> {
    let rows = excel::read_workbook(&path)?;
    let conn = state.db.lock().unwrap();
    let (mut imported, mut skipped) = (0, 0);
    for (date, huf) in rows {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM transactions WHERE date = ?1 AND currency = 'HUF' AND amount = ?2)",
                params![date.to_string(), huf as f64],
                |r| r.get(0),
            )
            .map_err(err)?;
        if exists {
            skipped += 1;
            continue;
        }
        conn.execute(
            "INSERT INTO transactions (date, currency, amount, rate, rate_source, rate_date, huf, note, source, kind)
             VALUES (?1, 'HUF', ?2, 1, '-', NULL, ?3, 'Excel import', 'excel', ?4)",
            params![date.to_string(), huf as f64, huf, manual_kind(huf as f64)],
        )
        .map_err(err)?;
        imported += 1;
    }
    Ok(ImportResult { imported, skipped })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct YearCount {
    year: i32,
    count: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportPreview {
    source: String,
    format: String,
    rows: Vec<import::ImportRow>,
    /// Ennyi összevont tétel már szerepel az adatbázisban (újraimportáláskor frissül).
    existing: usize,
    /// A kihagyott (nem fiat ↔ kripto) sorok száma.
    ignored: usize,
    warnings: Vec<String>,
    /// Kézzel vagy Excelből rögzített tételek az érintett években – duplán számolhatnak.
    manual_in_years: Vec<YearCount>,
}

/// Tőzsdei/banki export beolvasása előnézetre (még nem ment semmit).
#[tauri::command(async)]
fn preview_import(state: State<'_, AppState>, path: String) -> Result<ImportPreview, String> {
    preview_parsed(&state, import::parse_file(&path)?)
}

/// Kraken főkönyv lekérése csak olvasó API kulccsal, előnézetre. A kulcsot nem tároljuk.
#[tauri::command(async)]
fn preview_kraken_api(state: State<'_, AppState>, key: String, secret: String, since: String) -> Result<ImportPreview, String> {
    if key.trim().is_empty() || secret.trim().is_empty() {
        return Err("Add meg az API kulcsot és a titkot.".into());
    }
    let start = parse_date(&since)?.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    preview_parsed(&state, import::fetch_kraken(&key, &secret, start)?)
}

fn preview_parsed(state: &AppState, parsed: import::Parsed) -> Result<ImportPreview, String> {
    let mut rows = import::aggregate(parsed.source, &parsed.flows);
    if rows.is_empty() {
        return Err(format!(
            "A fájlban ({}) nem találtam fiat ↔ kripto ügyletet. {} sort hagytam ki (befizetés, kifizetés, kripto–kripto csere stb.).",
            parsed.format, parsed.ignored
        ));
    }
    let conn = state.db.lock().unwrap();
    let mut existing = 0;
    for r in &mut rows {
        let found: bool = conn
            .query_row("SELECT EXISTS(SELECT 1 FROM transactions WHERE external_id = ?1)", [&r.key], |x| x.get(0))
            .map_err(err)?;
        existing += usize::from(found);
        r.duplicate = find_duplicate(&conn, parsed.source, r)?;
    }
    let years: std::collections::BTreeSet<i32> = rows.iter().filter_map(|r| r.date[..4].parse().ok()).collect();
    let mut manual_in_years = Vec::new();
    for year in years {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE source IN ('kézi', 'excel') AND substr(date, 1, 4) = ?1",
                [year.to_string()],
                |x| x.get(0),
            )
            .map_err(err)?;
        if count > 0 {
            manual_in_years.push(YearCount { year, count });
        }
    }
    Ok(ImportPreview {
        source: parsed.source.into(),
        format: parsed.format.into(),
        rows,
        existing,
        ignored: parsed.ignored,
        warnings: parsed.warnings,
        manual_in_years,
    })
}

/// Másik forrásból (kézi, Excel, másik tőzsde) származó, ugyanazon a napon azonos irányú és
/// legfeljebb 1%-kal eltérő összegű tétel – valószínűleg ugyanaz az ügylet.
fn find_duplicate(conn: &Connection, source: &str, r: &import::ImportRow) -> Result<Option<String>, String> {
    let sign = if r.kind == "buy" { 1.0 } else { -1.0 };
    conn.query_row(
        "SELECT source, amount FROM transactions
         WHERE date = ?1 AND currency = ?2 AND source <> ?3 AND amount * ?4 > 0
           AND abs(abs(amount) - ?5) <= ?5 * 0.01
         LIMIT 1",
        params![r.date, r.currency, source, sign, r.amount],
        |x| Ok((x.get::<_, String>(0)?, x.get::<_, f64>(1)?)),
    )
    .optional()
    .map(|o| o.map(|(src, amount)| format!("{} tétel ugyanezen a napon: {} {}", capitalize(&src), hu_num(amount.abs(), 2), r.currency)))
    .map_err(err)
}

/// A kiválasztott összevont tételek mentése MNB árfolyammal. A már meglévő kulcsú tételeket frissíti.
#[tauri::command]
async fn commit_import(state: State<'_, AppState>, source: String, rows: Vec<import::ImportRow>) -> Result<ImportResult, String> {
    // Devizánként egyetlen MNB lekérés a teljes időszakra.
    let mut ranges: BTreeMap<String, (NaiveDate, NaiveDate)> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.currency != "HUF") {
        let d = parse_date(&r.date)?;
        let e = ranges.entry(r.currency.clone()).or_insert((d, d));
        e.0 = e.0.min(d);
        e.1 = e.1.max(d);
    }
    for (currency, (from, to)) in ranges {
        let cur = currency.clone();
        let days = tauri::async_runtime::spawn_blocking(move || mnb::fetch_range(&cur, from - Duration::days(14), to))
            .await
            .map_err(err)??;
        let conn = state.db.lock().unwrap();
        for (d, rate) in &days {
            conn.execute(
                "INSERT OR REPLACE INTO rates (currency, date, rate) VALUES (?1, ?2, ?3)",
                params![currency, d.to_string(), rate],
            )
            .map_err(err)?;
        }
    }

    let conn = state.db.lock().unwrap();
    let (mut imported, mut updated) = (0, 0);
    for r in &rows {
        let (rate, rate_source, rate_date): (f64, &str, Option<String>) = if r.currency == "HUF" {
            (1.0, "-", None)
        } else {
            let (d, rate): (String, f64) = conn
                .query_row(
                    "SELECT date, rate FROM rates WHERE currency = ?1 AND date <= ?2 ORDER BY date DESC LIMIT 1",
                    params![r.currency, r.date],
                    |x| Ok((x.get(0)?, x.get(1)?)),
                )
                .map_err(|_| format!("Nincs MNB {} árfolyam erre a napra: {}", r.currency, r.date))?;
            (rate, "MNB", Some(d))
        };
        // Vétel = kiadás (pozitív), eladás = bevétel (negatív), mint a kézi tételeknél.
        let amount = if r.kind == "buy" { r.amount } else { -r.amount };
        let huf = (amount * rate).round() as i64;
        let label = match r.kind.as_str() {
            "buy" => "vétel",
            _ => "eladás",
        };
        let mut note = format!("{} · {} {label}", capitalize(&source), r.count);
        if r.fee > 0.0 {
            note.push_str(&format!(" · díj {} {}", hu_num(r.fee, 2), r.currency));
        }
        let exists: bool = conn
            .query_row("SELECT EXISTS(SELECT 1 FROM transactions WHERE external_id = ?1)", [&r.key], |x| x.get(0))
            .map_err(err)?;
        conn.execute(
            UPSERT_IMPORTED,
            params![r.date, r.currency, amount, rate, rate_source, rate_date, huf, note, source, r.kind, r.fee, r.key, r.count as i64],
        )
        .map_err(err)?;
        if exists { updated += 1 } else { imported += 1 }
    }
    Ok(ImportResult { imported, skipped: updated })
}

const UPSERT_IMPORTED: &str = "INSERT INTO transactions
        (date, currency, amount, rate, rate_source, rate_date, huf, note, source, kind, fee, external_id, trades)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
    ON CONFLICT(external_id) DO UPDATE SET
        date = excluded.date, currency = excluded.currency, amount = excluded.amount, rate = excluded.rate,
        rate_source = excluded.rate_source, rate_date = excluded.rate_date, huf = excluded.huf,
        note = excluded.note, kind = excluded.kind, fee = excluded.fee, trades = excluded.trades";

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// Egy forrásból (pl. "binance") importált tételek törlése, opcionálisan egy évre szűkítve.
#[tauri::command]
fn delete_by_source(state: State<'_, AppState>, source: String, year: Option<i32>) -> Result<usize, String> {
    state
        .db
        .lock()
        .unwrap()
        .execute(
            "DELETE FROM transactions WHERE source = ?1 AND (?2 IS NULL OR substr(date, 1, 4) = ?2)",
            params![source, year.map(|y| y.to_string())],
        )
        .map_err(err)
}

fn kind_label(kind: &str, amount: f64) -> &'static str {
    match kind {
        "buy" => "Vétel",
        "sell" => "Eladás",
        "payment" => "Fizetés kriptóval",
        _ if amount > 0.0 => "Befizetés",
        _ => "Kivétel",
    }
}

fn year_remark(y: &tax::YearSummary) -> &'static str {
    if y.pre_regime {
        "2022-be számít"
    } else if y.loss_credit > 0 && !y.loss_declared {
        "veszteség nincs bevallva"
    } else {
        ""
    }
}

fn hu_num(x: f64, decimals: usize) -> String {
    format!("{x:.decimals$}").replace('.', ",")
}

/// CSV export (pontosvessző elválasztás, tizedesvessző, UTF-8 BOM – a magyar Excel így nyitja meg helyesen).
#[tauri::command(async)]
fn export_csv(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    let txs = all_transactions(&conn, None)?;
    let years = compute_summary(&conn)?;
    drop(conn);

    let clean = |s: &str| s.replace([';', '\n', '\r'], " ");
    let mut f = std::io::BufWriter::new(std::fs::File::create(&path).map_err(err)?);
    let mut w = || -> std::io::Result<()> {
        f.write_all(b"\xEF\xBB\xBF")?;
        writeln!(f, "Tranzakciók")?;
        writeln!(f, "Dátum;Típus;Forrás;Deviza;Összeg;Árfolyam;Árfolyam forrása;Árfolyam napja;Forint;Megjegyzés")?;
        for t in txs.iter().rev() {
            writeln!(
                f,
                "{};{};{};{};{};{};{};{};{};{}",
                t.date,
                kind_label(&t.kind, t.amount),
                t.source,
                t.currency,
                hu_num(t.amount, 2),
                hu_num(t.rate, 4),
                t.rate_source,
                t.rate_date.as_deref().unwrap_or(""),
                t.huf,
                clean(&t.note)
            )?;
        }
        writeln!(f)?;
        writeln!(f, "Éves összesítő")?;
        writeln!(f, "Év;Befizetve (Ft);Kivéve (Ft);Eredmény (Ft);Adó 15% (Ft);Adókiegyenlítés (Ft);Fizetendő adó, negatív = visszajár (Ft);Veszteség 15%-a (Ft);Ebből fel nem használt (Ft);Megjegyzés")?;
        for y in &years {
            let remark = year_remark(y);
            writeln!(
                f,
                "{};{};{};{};{};{};{};{};{};{}",
                y.year,
                y.deposits_huf,
                y.withdrawals_huf,
                y.result_huf,
                y.tax_gross,
                y.offset_used,
                y.tax_due,
                y.loss_credit,
                y.loss_credit_remaining,
                remark
            )?;
        }
        f.flush()
    };
    w().map_err(err)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let saved = std::fs::read_to_string(dir.join("current-profile")).unwrap_or_default();
            let profile =
                if data::profile_names(&dir).contains_key(saved.trim()) { saved.trim().to_string() } else { DEFAULT_PROFILE.to_string() };
            let conn = open_db(&db_path(&dir, &profile))?;
            // Heti automatikus mentés; ha nem sikerül, az app ettől még elindul.
            let _ = backup::auto(&conn, &dir.join("backups"), &profile, Local::now().date_naive());
            app.manage(AppState { db: Arc::new(Mutex::new(conn)), dir, profile: Mutex::new(profile) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_transactions,
            add_transaction,
            update_transaction,
            delete_transaction,
            get_mnb_rate,
            summary,
            preview_tax,
            import_excel,
            preview_import,
            preview_kraken_api,
            commit_import,
            delete_by_source,
            set_loss_declared,
            declaration_impact,
            export_csv,
            data::export_xlsx,
            data::export_pdf,
            data::save_text,
            data::file_dialog_ready,
            data::get_settings,
            data::set_setting,
            data::backup_save,
            data::backup_list,
            data::backup_dir,
            data::backup_is_encrypted,
            data::backup_restore,
            data::list_profiles,
            data::create_profile,
            data::switch_profile,
            data::rename_profile,
            data::delete_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_old_database_and_upserts_imports() {
        let conn = Connection::open_in_memory().unwrap();
        // A 0.1.0 előtti séma, néhány régi tétellel.
        conn.execute_batch(
            "CREATE TABLE transactions (id INTEGER PRIMARY KEY, date TEXT NOT NULL, currency TEXT NOT NULL,
                amount REAL NOT NULL, rate REAL NOT NULL, rate_source TEXT NOT NULL, rate_date TEXT,
                huf INTEGER NOT NULL, note TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
             INSERT INTO transactions (date, currency, amount, rate, rate_source, huf, note)
                VALUES ('2025-01-10', 'HUF', 131000, 1, '-', 131000, 'Excel import'),
                       ('2026-06-03', 'EUR', -3000, 384.2, 'MNB', -1152600, '');",
        )
        .unwrap();
        init_db(&conn).unwrap();
        init_db(&conn).unwrap(); // másodszor is lefut hiba nélkül

        let txs = all_transactions(&conn, None).unwrap();
        assert_eq!((txs[0].source.as_str(), txs[0].kind.as_str()), ("kézi", "withdrawal"));
        assert_eq!((txs[1].source.as_str(), txs[1].kind.as_str()), ("excel", "deposit"));

        let insert = |amount: f64| {
            conn.execute(
                UPSERT_IMPORTED,
                params!["2026-03-04", "EUR", amount, 390.0, "MNB", "2026-03-04", (amount * 390.0) as i64, "x", "binance", "buy", 0.4, "binance:2026-03-04:EUR:buy", 2],
            )
            .unwrap();
        };
        insert(400.4);
        insert(500.0); // újraimportálás: frissít, nem duplikál
        let imported: Vec<Tx> = all_transactions(&conn, Some(2026)).unwrap().into_iter().filter(|t| t.source == "binance").collect();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].amount, 500.0);
    }

    #[test]
    fn flags_same_day_entry_from_other_source() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn.execute(
            "INSERT INTO transactions (date, currency, amount, rate, rate_source, huf, source, kind)
             VALUES ('2026-03-04', 'EUR', 500, 390, 'MNB', 195000, 'kézi', 'deposit')",
            [],
        )
        .unwrap();
        let row = |kind: &str, amount: f64| import::ImportRow {
            key: String::new(),
            date: "2026-03-04".into(),
            currency: "EUR".into(),
            kind: kind.into(),
            amount,
            fee: 0.0,
            count: 1,
            duplicate: None,
        };
        assert!(find_duplicate(&conn, "binance", &row("buy", 502.0)).unwrap().is_some());
        assert!(find_duplicate(&conn, "binance", &row("buy", 600.0)).unwrap().is_none());
        assert!(find_duplicate(&conn, "binance", &row("sell", 500.0)).unwrap().is_none());
        assert!(find_duplicate(&conn, "kézi", &row("buy", 500.0)).unwrap().is_none());
    }

    #[test]
    fn crypto_payment_is_income() {
        let input = |amount: f64, kind: Option<&str>| TxInput {
            date: "2026-01-01".into(),
            currency: "EUR".into(),
            amount,
            rate: None,
            note: None,
            kind: kind.map(str::to_string),
        };
        assert_eq!(input(-20.0, Some("payment")).kind(), "payment");
        assert_eq!(input(20.0, Some("payment")).kind(), "deposit");
        assert_eq!(input(-20.0, None).kind(), "withdrawal");
    }
}
