//! Bybit exportok:
//! - Spot kereskedési előzmények (Orders → Spot Order → Trade History → Export):
//!   pár (pl. BTCEUR), irány, kitöltött érték, ár, mennyiség, díj, időpont (UTC);
//! - Convert előzmények (Orders → Convert → Export): miből, mennyit, mibe, mennyit, időpont.
//!
//! Csak a fiat ellenében kötött ügyletek számítanak (pl. BTC/EUR); a USDT-párok kripto–kripto
//! ügyletek, ezeket kihagyjuk.

use super::{FIAT, FiatFlow, Parsed, Table, budapest_date, cell, datetime, fiat, num};

const PAIR: &[&str] = &["Spot Pairs", "Spot Pair", "Symbol", "Trading Pair", "Pair", "Contracts"];
const SIDE: &[&str] = &["Direction", "Side", "Order Direction", "Trade Type", "Buy/Sell"];
const VALUE: &[&str] = &["Filled Value", "Exec Value", "Trade Value", "Order Value", "Total", "Value", "Amount"];
const PRICE: &[&str] = &["Filled Price", "Exec Price", "Avg. Filled Price", "Avg Filled Price", "Price", "Order Price"];
const QTY: &[&str] = &["Filled Quantity", "Filled Qty", "Exec Qty", "Quantity", "Qty"];
const FEE: &[&str] = &["Fees", "Trading Fee", "Fee", "Exec Fee", "Trading Fees"];
const FEE_CCY: &[&str] = &["Fee Currency", "Fee Coin", "Fee Asset"];
const TIME: &[&str] = &["Timestamp (UTC)", "Time(UTC)", "Trade Time(UTC)", "Transaction Time(UTC)", "Filled Time(UTC)", "Order Time", "Created Time", "Time", "Date"];

const FROM_CCY: &[&str] = &["From Currency", "From Coin", "From", "Sell Coin", "Pay Coin", "Spend Currency"];
const FROM_AMT: &[&str] = &["From Amount", "From Quantity", "Sell Amount", "Pay Amount", "Spend Amount"];
const TO_CCY: &[&str] = &["To Currency", "To Coin", "To", "Buy Coin", "Receive Coin", "Get Currency"];
const TO_AMT: &[&str] = &["To Amount", "To Quantity", "Buy Amount", "Receive Amount", "Get Amount"];

pub fn detect(t: &Table) -> bool {
    let trades = t.has(PAIR) && t.has(SIDE) && (t.has(VALUE) || (t.has(PRICE) && t.has(QTY)));
    let convert = t.has(FROM_CCY) && t.has(FROM_AMT) && t.has(TO_CCY) && t.has(TO_AMT);
    trades || convert
}

pub fn parse(t: &Table) -> Result<Parsed, String> {
    if t.has(FROM_CCY) && t.has(TO_CCY) { parse_convert(t) } else { parse_trades(t) }
}

/// "BTCEUR", "BTC/EUR", "btc-eur" → ("BTC", "EUR"), ha az ellenoldal fiat.
fn split_fiat_pair(pair: &str) -> Option<(String, String)> {
    let p: String = pair.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_uppercase();
    FIAT.iter()
        .find(|f| p.len() > f.len() && p.ends_with(*f))
        .map(|f| (p[..p.len() - f.len()].to_string(), f.to_string()))
}

fn parse_trades(t: &Table) -> Result<Parsed, String> {
    let pair = t.require(PAIR)?;
    let side = t.require(SIDE)?;
    let time = t.require(TIME)?;
    let (value, price, qty) = (t.col(VALUE), t.col(PRICE), t.col(QTY));
    let (fee, fee_ccy) = (t.col(FEE), t.col(FEE_CCY));
    let mut flows = Vec::new();
    let (mut ignored, mut fee_guess) = (0, 0);

    for row in &t.rows {
        let Some((base, quote)) = split_fiat_pair(cell(row, pair)) else {
            ignored += 1;
            continue;
        };
        let dir = cell(row, side).to_lowercase();
        let buy = dir.starts_with("buy") || dir == "b";
        if !(buy || dir.starts_with("sell") || dir == "s") {
            ignored += 1;
            continue;
        }
        let p = price.and_then(|i| num(cell(row, i)));
        let v = value
            .and_then(|i| num(cell(row, i)))
            .or_else(|| Some(p? * qty.and_then(|i| num(cell(row, i)))?));
        let (Some(v), Some(when)) = (v, datetime(cell(row, time))) else {
            ignored += 1;
            continue;
        };
        // A díj vagy fiatban, vagy (vételnél jellemzően) a megvett kriptóban van.
        let raw_fee = fee.and_then(|i| num(cell(row, i))).unwrap_or(0.0).abs();
        let fee_currency = fee_ccy.map(|i| cell(row, i).to_uppercase()).unwrap_or_default();
        let fee_fiat = if raw_fee == 0.0 || fiat(&fee_currency).is_some() {
            raw_fee
        } else if fee_currency == base || (fee_currency.is_empty() && buy) {
            fee_guess += usize::from(fee_currency.is_empty());
            raw_fee * p.unwrap_or(0.0)
        } else {
            raw_fee
        };
        let v = v.abs();
        let amount = if buy { -(v + fee_fiat) } else { v - fee_fiat };
        flows.push(FiatFlow { date: budapest_date(when), currency: quote, amount, fee: fee_fiat });
    }

    let mut warnings = Vec::new();
    if fee_guess > 0 {
        warnings.push(format!(
            "{fee_guess} vételnél a díj devizája nem szerepel; a megvett kriptóban számoltam, az ügylet árfolyamán átváltva."
        ));
    }
    Ok(Parsed { source: "bybit", format: "Bybit – spot kereskedések", flows, ignored, warnings })
}

fn parse_convert(t: &Table) -> Result<Parsed, String> {
    let (fc, fa, tc, ta) = (t.require(FROM_CCY)?, t.require(FROM_AMT)?, t.require(TO_CCY)?, t.require(TO_AMT)?);
    let time = t.require(TIME)?;
    let mut flows = Vec::new();
    let mut ignored = 0;

    for row in &t.rows {
        let Some(when) = datetime(cell(row, time)) else {
            ignored += 1;
            continue;
        };
        let date = budapest_date(when);
        let (from, to) = (fiat(cell(row, fc)), fiat(cell(row, tc)));
        match (from, to) {
            (Some(currency), None) => match num(cell(row, fa)) {
                Some(a) => flows.push(FiatFlow { date, currency, amount: -a.abs(), fee: 0.0 }),
                None => ignored += 1,
            },
            (None, Some(currency)) => match num(cell(row, ta)) {
                Some(a) => flows.push(FiatFlow { date, currency, amount: a.abs(), fee: 0.0 }),
                None => ignored += 1,
            },
            _ => ignored += 1,
        }
    }
    Ok(Parsed { source: "bybit", format: "Bybit – Convert", flows, ignored, warnings: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn spot_trades() {
        let csv = "\
Spot Pairs,Order Type,Direction,Filled Value,Filled Price,Filled Quantity,Fees,Fee Currency,Transaction ID,Order No.,Timestamp (UTC)
BTCEUR,Market,BUY,600,60000,0.01,0.00001,BTC,1,1,2026-04-01 08:00:00
BTCEUR,Limit,SELL,350,70000,0.005,0.35,EUR,2,2,2026-05-01 08:00:00
BTCUSDT,Market,BUY,1000,65000,0.0154,0.00001,BTC,3,3,2026-05-02 08:00:00
";
        let t = table_from_rows(read_csv(csv).unwrap()).unwrap();
        assert!(super::detect(&t));
        let p = super::parse(&t).unwrap();
        assert_eq!(p.ignored, 1);
        let rows = aggregate(p.source, &p.flows);
        assert_eq!((rows[0].kind.as_str(), rows[0].amount, rows[0].fee), ("buy", 600.6, 0.6));
        assert_eq!((rows[1].kind.as_str(), rows[1].amount), ("sell", 349.65));
    }

    #[test]
    fn convert() {
        let csv = "\
From Currency,From Amount,To Currency,To Amount,Rate,Time
EUR,200,BTC,0.003,0.000015,2026-06-10 12:00:00
ETH,1,EUR,2500,2500,2026-06-11 12:00:00
USDT,100,BTC,0.0015,0.000015,2026-06-12 12:00:00
";
        let t = table_from_rows(read_csv(csv).unwrap()).unwrap();
        assert!(super::detect(&t));
        let p = super::parse(&t).unwrap();
        let rows = aggregate(p.source, &p.flows);
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].kind.as_str(), rows[0].amount), ("buy", 200.0));
        assert_eq!((rows[1].kind.as_str(), rows[1].amount), ("sell", 2500.0));
    }
}
