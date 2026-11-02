//! Ellenőrzések és tervezés: hiányzó vagy gyanús tételek felderítése, illetve egy tervezett
//! kivétel adóhatása (most vagy jövőre), a kisértékű ügylet keretével.

use crate::{AppState, Hypothetical, compute_summary, compute_summary_with, err, hu_num, setting, tax};
use chrono::{Datelike, Duration, Local, NaiveDate};
use rusqlite::{Connection, params};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use tauri::State;

/// Importált forrás ennyi napnál hosszabb csend után „hézagot” jelez.
const GAP_DAYS: i64 = 180;
/// Az utolsó importált tétel ennyi napnál régebbi: lehet, hogy újra kell importálni.
const STALE_DAYS: i64 = 60;

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// "warn": valószínűleg hibás az adó; "info": érdemes ellenőrizni.
    level: &'static str,
    year: Option<i32>,
    text: String,
}

/// Hiányzó vagy hibás tételek felderítése (csak jelez, nem módosít semmit).
#[tauri::command]
pub fn find_issues(state: State<'_, AppState>) -> Result<Vec<Issue>, String> {
    issues(&state.db.lock().unwrap(), Local::now().date_naive())
}

fn source_name(s: &str) -> String {
    crate::capitalize(s)
}

fn year_of(date: &str) -> Option<i32> {
    date.get(..4)?.parse().ok()
}

pub(crate) fn issues(conn: &Connection, today: NaiveDate) -> Result<Vec<Issue>, String> {
    let mut out = Vec::new();

    // 1. Kivétel (bevétel) úgy, hogy előtte semmilyen befizetés vagy vétel nincs rögzítve.
    let first_in: Option<String> =
        conn.query_row("SELECT MIN(date) FROM transactions WHERE huf > 0", [], |r| r.get(0)).map_err(err)?;
    let early: Vec<(String, i64)> = conn
        .prepare("SELECT date, -huf FROM transactions WHERE huf < 0 AND (?1 IS NULL OR date < ?1) ORDER BY date")
        .and_then(|mut s| s.query_map([&first_in], |r| Ok((r.get(0)?, r.get(1)?)))?.collect())
        .map_err(err)?;
    if let Some((date, _)) = early.first() {
        let sum: i64 = early.iter().map(|(_, h)| h).sum();
        out.push(Issue {
            level: "warn",
            year: year_of(date),
            text: format!(
                "{} bevételt hozó tétel ({} Ft, az első: {date}) előtt nincs rögzített befizetés vagy vétel. Ha a kriptót korábban \
                 vetted, a vétel hiányzik – nélküle a teljes összeg nyereségnek számít.",
                early.len(),
                hu_thousands(sum)
            ),
        });
    }

    // 2. Ugyanarra az évre banki kivonat és tőzsdei import is van: a két forrás ugyanazt a pénzt mutathatja.
    let by_year: BTreeMap<i32, BTreeSet<String>> = conn
        .prepare("SELECT DISTINCT CAST(substr(date, 1, 4) AS INTEGER), source FROM transactions WHERE source NOT IN ('kézi', 'excel')")
        .and_then(|mut s| s.query_map([], |r| Ok((r.get::<_, i32>(0)?, r.get::<_, String>(1)?)))?.collect::<Result<Vec<_>, _>>())
        .map_err(err)?
        .into_iter()
        .fold(BTreeMap::new(), |mut m, (y, src)| {
            m.entry(y).or_insert_with(BTreeSet::new).insert(src);
            m
        });
    for (year, sources) in &by_year {
        if sources.contains("bank") && sources.len() > 1 {
            let others: Vec<String> = sources.iter().filter(|s| *s != "bank").map(|s| source_name(s)).collect();
            out.push(Issue {
                level: "warn",
                year: Some(*year),
                text: format!(
                    "{year}: banki kivonat és tőzsdei import ({}) is van. A banki utalás és a tőzsdei vétel ugyanaz a pénz lehet – \
                     ellenőrizd, hogy nem számol-e duplán.",
                    others.join(", ")
                ),
            });
        }
    }

    // 3. Már rögzített, valószínűleg dupla tételek: más forrás, azonos nap, deviza és irány, max. 1% eltérés.
    let dups: Vec<(String, String, String, f64, String)> = conn
        .prepare(
            "SELECT a.date, a.source, b.source, abs(a.amount), a.currency FROM transactions a JOIN transactions b
               ON a.date = b.date AND a.currency = b.currency AND a.id < b.id AND a.source <> b.source
              AND a.amount * b.amount > 0 AND abs(abs(a.amount) - abs(b.amount)) <= abs(a.amount) * 0.01
             ORDER BY a.date",
        )
        .and_then(|mut s| s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?.collect())
        .map_err(err)?;
    for (date, a, b, amount, currency) in dups {
        out.push(Issue {
            level: "warn",
            year: year_of(&date),
            text: format!(
                "{date}: {} és {} tétel ugyanarra a napra, közel azonos összeggel ({} {currency}) – valószínűleg ugyanaz az ügylet kétszer.",
                source_name(&a),
                source_name(&b),
                hu_num(amount, 2)
            ),
        });
    }

    // 4–5. Importált forrásonként: hosszú csend két tétel között, illetve régi utolsó import.
    let dated: Vec<(String, String)> = conn
        .prepare("SELECT DISTINCT source, date FROM transactions WHERE source NOT IN ('kézi', 'excel') ORDER BY source, date")
        .and_then(|mut s| s.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect())
        .map_err(err)?;
    let mut per_source: BTreeMap<String, Vec<NaiveDate>> = BTreeMap::new();
    for (src, d) in dated {
        if let Ok(d) = NaiveDate::parse_from_str(&d, "%Y-%m-%d") {
            per_source.entry(src).or_default().push(d);
        }
    }
    for (src, dates) in &per_source {
        for w in dates.windows(2) {
            if (w[1] - w[0]).num_days() > GAP_DAYS {
                out.push(Issue {
                    level: "info",
                    year: Some(w[0].year()),
                    text: format!(
                        "{}: {} és {} között nincs tétel. Ha ebben az időszakban is volt fiat ↔ kripto ügyleted, hiányozhat egy exportrész.",
                        source_name(src),
                        w[0],
                        w[1]
                    ),
                });
            }
        }
        if let Some(&last) = dates.last()
            && today - last > Duration::days(STALE_DAYS)
            && last.year() >= today.year() - 1
        {
            out.push(Issue {
                level: "info",
                year: Some(last.year()),
                text: format!(
                    "{}: az utolsó importált tétel {last}. Ha azóta is volt ügyleted, importáld újra (a meglévő napok frissülnek, nem duplázódnak).",
                    source_name(src)
                ),
            });
        }
    }
    Ok(out)
}

fn hu_thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    year: i32,
    /// A kivétel évének fizetendő adója (negatív: visszajár).
    tax_due_year: i64,
    /// Az alapévtől kezdve az összes év fizetendő adója együtt.
    tax_total: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SmallValue {
    /// Egy ügylet legfeljebb ennyi bevételig lehet kisértékű (a minimálbér 10%-a).
    day_limit: i64,
    /// Az éves felső határ (a minimálbér).
    year_cap: i64,
    /// Az évben eddig kisértékűnek minősülő bevétel.
    year_used: i64,
    /// Ma már volt bevételt hozó ügylet (akkor a mai újabb nem lehet kisértékű).
    today_income: bool,
    /// A tervezett kivétel kisértékű ügyletnek minősülhet (ha egyetlen ügyletként, ma történik).
    fits: bool,
    /// Be van-e kapcsolva a kisértékű szabály alkalmazása.
    enabled: bool,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    /// Kivétel nélkül, az alapévtől az összes év adója.
    baseline_total: i64,
    now: Scenario,
    next: Scenario,
    small: Option<SmallValue>,
}

/// „Mi lenne, ha” tervező: `huf` forint kivétel (bevétel) idén vagy jövőre, és az adóhatása.
#[tauri::command]
pub fn plan_withdrawal(state: State<'_, AppState>, year: i32, huf: i64) -> Result<Plan, String> {
    plan(&state.db.lock().unwrap(), year, huf, Local::now().date_naive())
}

pub(crate) fn plan(conn: &Connection, year: i32, huf: i64, today: NaiveDate) -> Result<Plan, String> {
    if huf <= 0 {
        return Err("Adj meg pozitív összeget.".into());
    }
    let total_from = |s: &[tax::YearSummary]| s.iter().filter(|y| y.year >= year).map(|y| y.tax_due).sum::<i64>();
    let scenario = |y: i32| -> Result<Scenario, String> {
        let s = compute_summary_with(conn, Some(Hypothetical { year: y, huf: -huf, exclude_id: None }))?;
        Ok(Scenario {
            year: y,
            tax_due_year: s.iter().find(|x| x.year == y).map(|x| x.tax_due).unwrap_or(0),
            tax_total: total_from(&s),
        })
    };
    let baseline_total = total_from(&compute_summary(conn)?);

    let small = match tax::minimum_wage(year) {
        Some(wage) => {
            let day_limit = wage / 10;
            let days: Vec<tax::SaleDay> = conn
                .prepare(
                    "SELECT CAST(substr(date, 1, 4) AS INTEGER), SUM(trades), SUM(-huf)
                     FROM transactions WHERE huf < 0 AND substr(date, 1, 4) = ?1 GROUP BY date",
                )
                .and_then(|mut s| {
                    s.query_map([year.to_string()], |r| Ok(tax::SaleDay { year: r.get(0)?, trades: r.get(1)?, huf: r.get(2)? }))?
                        .collect()
                })
                .map_err(err)?;
            let year_used: i64 = days.iter().filter(|d| d.trades == 1 && d.huf * 10 <= wage).map(|d| d.huf).sum();
            let today_income: bool = conn
                .query_row("SELECT EXISTS(SELECT 1 FROM transactions WHERE huf < 0 AND date = ?1)", params![today.to_string()], |r| {
                    r.get(0)
                })
                .map_err(err)?;
            let fits = year == today.year() && huf <= day_limit && !today_income && year_used + huf <= wage;
            let enabled = setting(conn, "kiserteku")?.as_deref() == Some("on");
            Some(SmallValue { day_limit, year_cap: wage, year_used, today_income, fits, enabled })
        }
        None => None,
    };

    Ok(Plan { baseline_total, now: scenario(year)?, next: scenario(year + 1)?, small })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_db;

    fn db(rows: &[(&str, i64, &str)]) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        for (date, huf, source) in rows {
            conn.execute(
                "INSERT INTO transactions (date, currency, amount, rate, rate_source, huf, source, kind)
                 VALUES (?1, 'HUF', ?2, 1, '-', ?2, ?3, '')",
                params![date, huf, source],
            )
            .unwrap();
        }
        conn
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
    }

    #[test]
    fn flags_income_without_purchase() {
        let conn = db(&[("2026-02-01", -100_000, "kézi"), ("2026-03-01", 50_000, "kézi")]);
        let found = issues(&conn, today()).unwrap();
        assert!(found.iter().any(|i| i.level == "warn" && i.text.contains("2026-02-01")));
        let clean = db(&[("2026-01-01", 50_000, "kézi"), ("2026-02-01", -100_000, "kézi")]);
        assert!(issues(&clean, today()).unwrap().is_empty());
    }

    #[test]
    fn flags_bank_and_exchange_and_duplicates_and_gaps() {
        let conn = db(&[
            ("2025-01-10", 100_000, "bank"),
            ("2025-01-10", 100_500, "binance"),
            ("2025-09-01", 10_000, "binance"),
            ("2026-01-05", 5_000, "kraken"),
        ]);
        let found = issues(&conn, today()).unwrap();
        assert!(found.iter().any(|i| i.text.contains("banki kivonat és tőzsdei import")));
        assert!(found.iter().any(|i| i.text.contains("kétszer")));
        assert!(found.iter().any(|i| i.text.starts_with("Binance: 2025-01-10 és 2025-09-01")));
        assert!(found.iter().any(|i| i.text.starts_with("Kraken: az utolsó importált tétel 2026-01-05")));
    }

    #[test]
    fn plans_withdrawal_now_or_next_year() {
        // 2026-ban 1 000 000 Ft befizetés, 1 200 000 Ft kivétel: 200 000 Ft nyereség.
        let conn = db(&[("2026-01-10", 1_000_000, "kézi"), ("2026-06-10", -1_200_000, "kézi")]);
        let p = plan(&conn, 2026, 100_000, today()).unwrap();
        assert_eq!(p.baseline_total, 30_000);
        assert_eq!(p.now.tax_total, 45_000); // +100 000 Ft nyereség 15%-a
        assert_eq!(p.next.tax_total, 45_000); // jövőre is ugyanennyi, csak egy évvel később esedékes
        assert_eq!(p.next.tax_due_year, 15_000);
        let s = p.small.unwrap();
        assert_eq!(s.day_limit, 32_280);
        assert!(!s.fits);
        assert!(plan(&conn, 2026, 30_000, today()).unwrap().small.unwrap().fits);
    }
}
