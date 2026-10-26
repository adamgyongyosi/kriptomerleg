//! Kraken „Ledgers” export (History → Export → Ledgers).
//! Oszlopok: txid, refid, time, type, subtype, aclass, asset, wallet, amount, fee, balance.
//!
//! Egy kereskedésnek két sora van (fiat és kripto, közös refid); a fiat sor nettó változása
//! (amount − fee) a kiadás vagy bevétel.

use super::{FiatFlow, Parsed, Table, budapest_date, cell, datetime, fiat, num};

const TRADE_TYPES: &[&str] = &["trade", "spend", "receive", "sale", "purchase", "settled"];

pub fn detect(t: &Table) -> bool {
    t.has(&["refid"]) && t.has(&["asset"]) && t.has(&["amount"]) && t.has(&["type"])
}

pub fn parse(t: &Table) -> Result<Parsed, String> {
    let ti = t.require(&["time"])?;
    let ty = t.require(&["type"])?;
    let asset = t.require(&["asset"])?;
    let amt = t.require(&["amount"])?;
    let fee_col = t.col(&["fee"]);
    let mut flows = Vec::new();
    let mut ignored = 0;

    for row in &t.rows {
        let kind = cell(row, ty).to_lowercase();
        let Some(currency) = fiat(cell(row, asset)) else {
            ignored += 1;
            continue;
        };
        let (Some(amount), Some(when)) = (num(cell(row, amt)), datetime(cell(row, ti))) else {
            ignored += 1;
            continue;
        };
        if !TRADE_TYPES.contains(&kind.as_str()) {
            ignored += 1;
            continue;
        }
        let fee = fee_col.and_then(|i| num(cell(row, i))).unwrap_or(0.0).abs();
        let net = amount - fee;
        if net == 0.0 {
            continue;
        }
        flows.push(FiatFlow { date: budapest_date(when), currency, amount: net, fee });
    }
    Ok(Parsed { source: "kraken", format: "Kraken – Ledgers", flows, ignored, warnings: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::super::*;

    const SAMPLE: &str = r#""txid","refid","time","type","subtype","aclass","asset","wallet","amount","fee","balance"
"L1","Q1","2026-02-10 09:00:00","deposit","","currency","ZEUR","spot / main",1000.0000,0.0000,1000.0000
"L2","T1","2026-02-10 09:10:00","trade","","currency","ZEUR","spot / main",-500.0000,1.3000,498.7000
"L3","T1","2026-02-10 09:10:00","trade","","currency","XXBT","spot / main",0.0060000000,0.0000000000,0.0060000000
"L4","T2","2026-09-01 15:00:00","trade","","currency","ZEUR","spot / main",300.0000,0.7800,797.9200
"L5","W1","2026-09-02 10:00:00","withdrawal","","currency","ZEUR","spot / main",-700.0000,0.0900,97.8300
"#;

    #[test]
    fn parses_ledger() {
        let t = table_from_rows(read_csv(SAMPLE).unwrap()).unwrap();
        assert!(super::detect(&t));
        let p = super::parse(&t).unwrap();
        let rows = aggregate(p.source, &p.flows);
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].kind.as_str(), rows[0].amount, rows[0].fee), ("buy", 501.3, 1.3));
        assert_eq!((rows[1].kind.as_str(), rows[1].amount), ("sell", 299.22));
    }
}
