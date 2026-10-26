//! A korábbi "crypto deposit.xlsx" beolvasása: minden évszám nevű lap A oszlopa a dátum, B oszlopa a forintösszeg.

use calamine::{Data, Reader, open_workbook_auto};
use chrono::{Duration, NaiveDate, NaiveDateTime};

pub fn read_workbook(path: &str) -> Result<Vec<(NaiveDate, i64)>, String> {
    let mut wb = open_workbook_auto(path).map_err(|e| format!("Nem sikerült megnyitni: {e}"))?;
    let mut out = Vec::new();
    for name in wb.sheet_names().to_owned() {
        let is_year = name.trim().parse::<i32>().is_ok_and(|y| (1900..2200).contains(&y));
        if !is_year {
            continue;
        }
        let range = wb.worksheet_range(&name).map_err(|e| format!("{name}: {e}"))?;
        for row in range.rows() {
            let (Some(date), Some(amount)) = (row.first().and_then(cell_date), row.get(1).and_then(cell_number)) else {
                continue;
            };
            let huf = amount.round() as i64;
            if huf != 0 {
                out.push((date, huf));
            }
        }
    }
    Ok(out)
}

fn excel_serial_to_date(serial: f64) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(1899, 12, 30)?.checked_add_signed(Duration::days(serial.floor() as i64))
}

fn cell_date(c: &Data) -> Option<NaiveDate> {
    match c {
        Data::DateTime(dt) => excel_serial_to_date(dt.as_f64()),
        Data::Float(f) => excel_serial_to_date(*f),
        Data::Int(i) => excel_serial_to_date(*i as f64),
        Data::DateTimeIso(s) | Data::String(s) => {
            let s = s.trim();
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .or_else(|_| NaiveDate::parse_from_str(s, "%Y.%m.%d"))
                .or_else(|_| NaiveDate::parse_from_str(s, "%Y. %m. %d."))
                .ok()
                .or_else(|| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").ok().map(|d| d.date()))
        }
        _ => None,
    }
}

fn cell_number(c: &Data) -> Option<f64> {
    match c {
        Data::Float(f) => Some(*f),
        Data::Int(i) => Some(*i as f64),
        Data::String(s) => s.trim().replace([' ', '\u{a0}'], "").replace(',', ".").parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    /// `KRIPTO_XLSX=<útvonal> cargo test -- --ignored excel`
    #[test]
    #[ignore]
    fn reads_real_workbook() {
        let path = std::env::var("KRIPTO_XLSX").expect("KRIPTO_XLSX");
        let rows = super::read_workbook(&path).unwrap();
        let total: i64 = rows.iter().map(|r| r.1).sum();
        println!("{} sor, összesen {total} Ft, első: {:?}, utolsó: {:?}", rows.len(), rows.first(), rows.last());
        assert!(!rows.is_empty());
    }
}
