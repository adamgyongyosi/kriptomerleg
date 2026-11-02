//! Crypto.com App export (Accounts → Transaction History → Export, „Crypto Wallet”):
//! Timestamp (UTC), Transaction Description, Currency, Amount, To Currency, To Amount,
//! Native Currency, Native Amount, Native Amount (in USD), Transaction Kind[, Transaction Hash].
//!
//! Fiat ↔ kripto ügyletek:
//! - `card_top_up`: kriptóból feltöltött Visa kártya – a kripto ekkor válik fiattá, ez a bevétel
//!   („Fizetés kriptóval” tételként, a Native Amount összegével);
//! - fiat → kripto (pl. `viban_purchase`): Currency fiat, To Currency kripto – kiadás;
//! - kripto → fiat (pl. `crypto_viban_exchange`): Currency kripto, To Currency fiat – bevétel;
//! - `crypto_purchase`: bankkártyával vett kripto – kiadás a Native Amount összegével.
//!
//! A cashback, a visszatérítés (`reimbursement`), a fiatból töltött kártya (`viban_card_top_up`),
//! a kripto–kripto csere, a be- és kiutalás nem kriptós ügylet, ezeket kihagyjuk.

use super::{FiatFlow, Parsed, Table, budapest_date, cell, datetime, fiat, num};

const KIND: &[&str] = &["Transaction Kind"];
const NATIVE: &[&str] = &["Native Amount"];

pub fn detect(t: &Table) -> bool {
    t.has(KIND) && t.has(NATIVE) && t.has(&["Native Currency"]) && t.has(&["To Currency"])
}

pub fn parse(t: &Table) -> Result<Parsed, String> {
    let time = t.require(&["Timestamp (UTC)", "Timestamp"])?;
    let (cur, amt) = (t.require(&["Currency"])?, t.require(&["Amount"])?);
    let (to_cur, to_amt) = (t.require(&["To Currency"])?, t.require(&["To Amount"])?);
    let (native_cur, native_amt) = (t.require(&["Native Currency"])?, t.require(NATIVE)?);
    let kind_col = t.require(KIND)?;
    let mut flows = Vec::new();
    let (mut ignored, mut top_ups) = (0, 0);

    for row in &t.rows {
        let kind = cell(row, kind_col).to_lowercase();
        let Some(when) = datetime(cell(row, time)) else {
            ignored += 1;
            continue;
        };
        let date = budapest_date(when);
        let (from_fiat, to_fiat) = (fiat(cell(row, cur)), fiat(cell(row, to_cur)));
        let native = || Some((fiat(cell(row, native_cur))?, num(cell(row, native_amt))?.abs()));
        // (deviza, előjeles fiat összeg, kártyás fizetés)
        let flow = match kind.as_str() {
            "card_top_up" if from_fiat.is_none() => {
                top_ups += 1;
                native().map(|(c, a)| (c, a, true))
            }
            "crypto_purchase" if from_fiat.is_none() => native().map(|(c, a)| (c, -a, false)),
            _ => match (from_fiat, to_fiat, cell(row, to_cur).trim().is_empty()) {
                // fiat → kripto: a kiadott fiat
                (Some(c), None, false) => num(cell(row, amt)).map(|a| (c, -a.abs(), false)),
                // kripto → fiat: a kapott fiat
                (None, Some(c), _) => num(cell(row, to_amt)).map(|a| (c, a.abs(), false)),
                _ => None,
            },
        };
        match flow {
            Some((currency, amount, payment)) if amount != 0.0 => {
                flows.push(FiatFlow { date, currency, amount, fee: 0.0, raw: row.join(";"), payment })
            }
            _ => ignored += 1,
        }
    }

    let mut warnings = Vec::new();
    if top_ups > 0 {
        warnings.push(format!(
            "{top_ups} kártyafeltöltést kriptóból („card_top_up”) „Fizetés kriptóval” tételként veszek át: a kripto ekkor vált fiattá. \
             A kártyás vásárlások maguk (fiatból) nem kriptós ügyletek. Ha a kártya exportját (card_transactions_record) is \
             importálnád, ugyanezek a feltöltések ott is szerepelnek – elég az egyik."
        ));
    }
    Ok(Parsed { source: "cryptocom", format: "Crypto.com – App tranzakciók", flows, ignored, warnings })
}

#[cfg(test)]
mod tests {
    use super::super::*;

    const SAMPLE: &str = "\
Timestamp (UTC),Transaction Description,Currency,Amount,To Currency,To Amount,Native Currency,Native Amount,Native Amount (in USD),Transaction Kind,Transaction Hash
2026-02-25 01:32:11,BTC -> EUR,BTC,-0.0015,,,EUR,125.0,135.6,card_top_up,
2026-02-26 09:00:00,Buy BTC,EUR,-200,BTC,0.0024,EUR,200,217,viban_purchase,
2026-03-01 10:00:00,BTC -> EUR,BTC,-0.001,EUR,85.5,EUR,85.5,92.7,crypto_viban_exchange,
2026-03-02 11:00:00,Buy ETH,ETH,0.05,,,EUR,110,119,crypto_purchase,
2026-03-03 12:00:00,Card Cashback,CRO,2.5,,,EUR,0.2,0.22,referral_card_cashback,
2026-03-04 12:00:00,EUR -> Card,EUR,-50,,,EUR,50,54,viban_card_top_up,
2026-03-05 12:00:00,BTC -> ETH,BTC,-0.001,ETH,0.03,EUR,85,92,crypto_exchange,
";

    #[test]
    fn card_top_up_is_crypto_payment() {
        let t = table_from_rows(read_csv(SAMPLE).unwrap()).unwrap();
        assert!(super::detect(&t));
        let p = super::parse(&t).unwrap();
        assert_eq!(p.ignored, 3);
        let rows = aggregate(p.source, &p.flows);
        let pay = rows.iter().find(|r| r.kind == "payment").unwrap();
        assert_eq!((pay.date.as_str(), pay.amount, pay.key.as_str()), ("2026-02-25", 125.0, "cryptocom:2026-02-25:EUR:payment"));
        let buys: Vec<f64> = rows.iter().filter(|r| r.kind == "buy").map(|r| r.amount).collect();
        assert_eq!(buys, [200.0, 110.0]);
        let sell = rows.iter().find(|r| r.kind == "sell").unwrap();
        assert_eq!((sell.date.as_str(), sell.amount), ("2026-03-01", 85.5));
    }
}
