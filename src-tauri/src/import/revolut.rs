//! Revolut kripto kivonat (Crypto → … → Statement, CSV vagy Excel).
//!
//! Két ismert formátum:
//! - újabb: Symbol, Type, Quantity, Price, Value, Fees, Date – a Type "Buy" / "Sell",
//!   a Value és Fees pénzformátumú ("€1,000.00");
//! - régebbi: Type, Product, Started Date, Completed Date, Description, Amount, Currency,
//!   Fiat amount, Fiat amount (inc. fees), Fee, Base currency, State, Balance.
//!
//! A Revolut helyi időt ír, ezért itt nincs időzóna-átszámítás.

use super::{FiatFlow, Parsed, Table, cell, currency_in, datetime, fiat, num};

const NEW_VALUE: &[&str] = &["Value"];
const OLD_FIAT: &[&str] = &["Fiat amount"];

pub fn detect(t: &Table) -> bool {
    (t.has(&["Symbol"]) && t.has(&["Type"]) && t.has(NEW_VALUE)) || (t.has(OLD_FIAT) && t.has(&["Base currency"]))
}

pub fn parse(t: &Table) -> Result<Parsed, String> {
    if t.has(OLD_FIAT) { parse_old(t) } else { parse_new(t) }
}

fn parse_new(t: &Table) -> Result<Parsed, String> {
    let ty = t.require(&["Type"])?;
    let value = t.require(NEW_VALUE)?;
    let fees = t.col(&["Fees", "Fee"]);
    let date = t.require(&["Date", "Completed Date", "Time"])?;
    let mut flows = Vec::new();
    let mut ignored = 0;

    for row in &t.rows {
        let kind = cell(row, ty).to_lowercase();
        let buy = kind == "buy";
        if !(buy || kind == "sell") {
            ignored += 1;
            continue;
        }
        let raw = cell(row, value);
        let (Some(v), Some(currency), Some(when)) = (num(raw), currency_in(raw), datetime(cell(row, date))) else {
            ignored += 1;
            continue;
        };
        let fee = fees.and_then(|i| num(cell(row, i))).unwrap_or(0.0).abs();
        let v = v.abs();
        let amount = if buy { -(v + fee) } else { v - fee };
        flows.push(FiatFlow { date: when.date(), currency, amount, fee });
    }
    Ok(Parsed { source: "revolut", format: "Revolut – kripto kivonat", flows, ignored, warnings: Vec::new() })
}

fn parse_old(t: &Table) -> Result<Parsed, String> {
    let ty = t.require(&["Type"])?;
    let date = t.require(&["Completed Date", "Started Date", "Date"])?;
    let amount_col = t.require(&["Amount"])?;
    let currency_col = t.require(&["Currency"])?;
    let fiat_amount = t.require(OLD_FIAT)?;
    let fiat_inc = t.col(&["Fiat amount (inc. fees)"]);
    let base = t.require(&["Base currency"])?;
    let fee_col = t.col(&["Fee"]);
    let state = t.col(&["State"]);
    let mut flows = Vec::new();
    let mut ignored = 0;

    for row in &t.rows {
        let completed = state.is_none_or(|i| cell(row, i).eq_ignore_ascii_case("completed"));
        if !completed || !cell(row, ty).eq_ignore_ascii_case("exchange") {
            ignored += 1;
            continue;
        }
        let (Some(amount), Some(when)) = (num(cell(row, amount_col)), datetime(cell(row, date))) else {
            ignored += 1;
            continue;
        };
        if let Some(currency) = fiat(cell(row, currency_col)) {
            // Fiat számla kivonata: az összeg maga a fiat mozgás.
            let fee = fee_col.and_then(|i| num(cell(row, i))).unwrap_or(0.0).abs();
            flows.push(FiatFlow { date: when.date(), currency, amount: amount - fee, fee });
            continue;
        }
        // Kripto számla kivonata: + mennyiség = vétel, − = eladás; a fiat érték a Base currency-ben.
        let Some(currency) = fiat(cell(row, base)) else {
            ignored += 1;
            continue;
        };
        let gross = num(cell(row, fiat_amount)).unwrap_or(0.0).abs();
        let inc = fiat_inc.and_then(|i| num(cell(row, i))).map(f64::abs).unwrap_or(gross);
        let fee = (inc - gross).abs();
        let amount = if amount > 0.0 { -(gross + fee) } else { gross - fee };
        flows.push(FiatFlow { date: when.date(), currency, amount, fee });
    }
    Ok(Parsed { source: "revolut", format: "Revolut – kripto kivonat (régi)", flows, ignored, warnings: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn new_format() {
        let csv = "\
Symbol,Type,Quantity,Price,Value,Fees,Date
BTC,Buy,0.01,\"€60,000.00\",\"€600.00\",€8.97,\"Jan 5, 2026, 10:23:45 AM\"
BTC,Sell,0.005,\"€70,000.00\",\"€350.00\",€5.24,\"Mar 2, 2026, 4:00:00 PM\"
ETH,Receive,1,,,,\"Mar 3, 2026, 4:00:00 PM\"
";
        let t = table_from_rows(read_csv(csv).unwrap()).unwrap();
        assert!(super::detect(&t));
        let p = super::parse(&t).unwrap();
        assert_eq!(p.ignored, 1);
        let rows = aggregate(p.source, &p.flows);
        assert_eq!((rows[0].kind.as_str(), rows[0].amount, rows[0].date.as_str()), ("buy", 608.97, "2026-01-05"));
        assert_eq!((rows[1].kind.as_str(), rows[1].amount), ("sell", 344.76));
    }

    #[test]
    fn old_format() {
        let csv = "\
Type,Product,Started Date,Completed Date,Description,Amount,Currency,Fiat amount,Fiat amount (inc. fees),Fee,Base currency,State,Balance
EXCHANGE,Current,2025-04-01 10:00:00,2025-04-01 10:00:01,Exchanged to BTC,0.002,BTC,120.00,121.79,0.00003,EUR,COMPLETED,0.002
EXCHANGE,Current,2025-05-01 10:00:00,2025-05-01 10:00:01,Exchanged to EUR,-0.001,BTC,-70.00,-68.95,0.000015,EUR,COMPLETED,0.001
TRANSFER,Current,2025-05-02 10:00:00,2025-05-02 10:00:01,To EUR,-0.001,BTC,-70.00,-70.00,0,EUR,COMPLETED,0
";
        let t = table_from_rows(read_csv(csv).unwrap()).unwrap();
        assert!(super::detect(&t));
        let rows = aggregate("revolut", &super::parse(&t).unwrap().flows);
        assert_eq!((rows[0].kind.as_str(), rows[0].amount), ("buy", 121.79));
        assert_eq!((rows[1].kind.as_str(), rows[1].amount), ("sell", 68.95));
    }
}
