//! Binance „Transaction History” export (Wallet → Transaction History → Export).
//! Oszlopok: User_ID, UTC_Time, Account, Operation, Coin, Change, Remark.
//!
//! Minden fiat-egyenleg változás, ami ügyletből (vétel, eladás, konvertálás, díj) jön, adóügyi
//! kiadás vagy bevétel; a be- és kifizetések, átvezetések nem.

use super::{FiatFlow, Parsed, Table, budapest_date, cell, datetime, fiat, num};

const TIME: &[&str] = &["UTC_Time", "Date(UTC)", "Time", "Date"];
const OPERATION: &[&str] = &["Operation", "Type"];
const COIN: &[&str] = &["Coin", "Asset"];
const CHANGE: &[&str] = &["Change", "Amount"];

/// Ezek a műveletek csak pénzmozgások, nem ügyletek.
const TRANSFER: &[&str] = &["deposit", "withdraw", "transfer", "send", "receive", "distribution", "airdrop", "savings", "earn", "staking", "launchpool", "cashback", "referral", "commission rebate"];
/// Ezek a műveletek fiat ↔ kripto ügyletek.
const TRADE: &[&str] = &["transaction", "trade", "convert", "buy", "sell", "fee", "spend", "revenue", "sold", "order", "otc"];

pub fn detect(t: &Table) -> bool {
    t.has(&["Operation"]) && t.has(COIN) && t.has(&["Change"])
}

pub fn parse(t: &Table) -> Result<Parsed, String> {
    let (ti, op, coin, ch) = (t.require(TIME)?, t.require(OPERATION)?, t.require(COIN)?, t.require(CHANGE)?);
    let mut flows = Vec::new();
    let (mut ignored, mut card, mut p2p) = (0, 0, 0);

    for row in &t.rows {
        let operation = cell(row, op).to_lowercase();
        let Some(currency) = fiat(cell(row, coin)) else {
            if operation.contains("buy crypto") || operation.contains("card") {
                card += 1;
            }
            if operation.contains("p2p") {
                p2p += 1;
            }
            ignored += 1;
            continue;
        };
        let is_trade = TRADE.iter().any(|k| operation.contains(k)) && !TRANSFER.iter().any(|k| operation.contains(k));
        let (Some(amount), Some(when)) = (num(cell(row, ch)), datetime(cell(row, ti))) else {
            ignored += 1;
            continue;
        };
        if !is_trade || amount == 0.0 {
            ignored += 1;
            continue;
        }
        let fee = if operation.contains("fee") { amount.abs() } else { 0.0 };
        flows.push(FiatFlow { date: budapest_date(when), currency, amount, fee });
    }

    let mut warnings = Vec::new();
    if card > 0 {
        warnings.push(format!(
            "{card} kártyás vásárlás („Buy Crypto”) sort találtam fiat összeg nélkül. Ha a kártyás fizetés nem jelenik meg euróban a listában, azt kézzel rögzítsd (vétel)."
        ));
    }
    if p2p > 0 {
        warnings.push(format!(
            "{p2p} P2P sort találtam. A P2P-ben a fiat a bankszámládon mozog, nem a Binance-en, ezért ezeket kézzel rögzítsd."
        ));
    }
    Ok(Parsed { source: "binance", format: "Binance – Transaction History", flows, ignored, warnings })
}

#[cfg(test)]
mod tests {
    use super::super::*;

    const SAMPLE: &str = "\
User_ID,UTC_Time,Account,Operation,Coin,Change,Remark
1,2026-03-04 10:00:00,Spot,Deposit,EUR,500,
1,2026-03-04 10:05:00,Spot,Transaction Spend,EUR,-400,
1,2026-03-04 10:05:00,Spot,Transaction Buy,BTC,0.005,
1,2026-03-04 10:05:00,Spot,Transaction Fee,EUR,-0.4,
1,2026-06-01 12:00:00,Spot,Transaction Sold,BTC,-0.002,
1,2026-06-01 12:00:00,Spot,Transaction Revenue,EUR,210.5,
1,2026-06-01 12:00:00,Spot,Transaction Spend,USDT,-100,
1,2026-06-02 08:00:00,Spot,Fiat Withdraw,EUR,-200,
1,2026-06-03 09:00:00,Spot,Binance Convert,EUR,-50,
";

    #[test]
    fn parses_fiat_trades_only() {
        let t = table_from_rows(read_csv(SAMPLE).unwrap()).unwrap();
        assert!(super::detect(&t));
        let p = super::parse(&t).unwrap();
        let rows = aggregate(p.source, &p.flows);
        let buy0304 = rows.iter().find(|r| r.date == "2026-03-04" && r.kind == "buy").unwrap();
        assert_eq!((buy0304.amount, buy0304.fee, buy0304.count), (400.4, 0.4, 2));
        let sell = rows.iter().find(|r| r.kind == "sell").unwrap();
        assert_eq!((sell.date.as_str(), sell.amount), ("2026-06-01", 210.5));
        assert!(rows.iter().any(|r| r.date == "2026-06-03" && r.amount == 50.0));
        // Befizetés, kifizetés, USDT és BTC sorok kimaradnak.
        assert_eq!(rows.len(), 3);
    }
}
