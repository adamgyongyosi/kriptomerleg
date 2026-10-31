//! Banki kivonat (Revolut, Wise, magyar bankok CSV/Excel exportja): a kriptotőzsdékre küldött és
//! onnan érkező utalások. Annak hasznos, aki a tőzsdei exportot nem tudja letölteni.
//!
//! A tőzsdére utalt összeg csak közelítése a kriptóra fordított kiadásnak (a tőzsdén maradhat
//! egyenleg), ezért ha van tőzsdei export, az pontosabb. Kiutalás a bankból = befizetés (kiadás),
//! jóváírás a tőzsdéről = kivétel (bevétel).

use super::{FiatFlow, Parsed, Table, cell, currency_in, datetime, fiat, num};

const DATE: &[&str] = &[
    "Completed Date",
    "Started Date",
    "Date",
    "Dátum",
    "Könyvelés dátuma",
    "Könyvelési dátum",
    "Tranzakció dátuma",
    "Értéknap",
    "Booking date",
];
const AMOUNT: &[&str] = &["Amount", "Összeg", "Terhelés/Jóváírás összege", "Tranzakció összege", "Value"];
const CURRENCY: &[&str] = &["Currency", "Pénznem", "Deviza", "Devizanem"];
const TEXT: &[&str] = &[
    "Description",
    "Közlemény",
    "Partner neve",
    "Ellenoldali név",
    "Kedvezményezett neve",
    "Partner",
    "Megjegyzés",
    "Payee Name",
    "Payer Name",
    "Merchant",
    "Payment Reference",
    "Tranzakció leírása",
];
const FEE: &[&str] = &["Fee", "Díj"];
const STATE: &[&str] = &["State", "Állapot"];

/// Kriptotőzsdék és fizetési partnereik nevei a közleményben / partnernévben (kisbetűvel).
const EXCHANGES: &[(&str, &str)] = &[
    ("binance", "Binance"),
    ("kraken", "Kraken"),
    ("payward", "Kraken"),
    ("bybit", "Bybit"),
    ("coinbase", "Coinbase"),
    ("bitstamp", "Bitstamp"),
    ("bitpanda", "Bitpanda"),
    ("okx", "OKX"),
    ("kucoin", "KuCoin"),
    ("crypto.com", "Crypto.com"),
    ("foris dax", "Crypto.com"),
    ("bitvavo", "Bitvavo"),
    ("gemini", "Gemini"),
    ("moonpay", "MoonPay"),
];

pub fn detect(t: &Table) -> bool {
    t.has(DATE) && t.has(AMOUNT) && TEXT.iter().any(|c| t.has(&[c]))
}

pub fn parse(t: &Table) -> Result<Parsed, String> {
    let date = t.require(DATE)?;
    let amount = t.require(AMOUNT)?;
    let currency = t.col(CURRENCY);
    let fee = t.col(FEE);
    let state = t.col(STATE);
    let texts: Vec<usize> = TEXT.iter().filter_map(|c| t.col(&[c])).collect();
    let mut flows = Vec::new();
    let mut ignored = 0;
    let mut matched: Vec<&str> = Vec::new();

    for row in &t.rows {
        let text: String = texts.iter().map(|&i| cell(row, i).to_lowercase()).collect::<Vec<_>>().join(" ");
        let Some(&(_, name)) = EXCHANGES.iter().find(|(k, _)| text.contains(k)) else {
            ignored += 1;
            continue;
        };
        if let Some(s) = state {
            let s = cell(row, s).to_lowercase();
            if !(s.is_empty() || s == "completed" || s.starts_with("teljes") || s == "könyvelt") {
                ignored += 1;
                continue;
            }
        }
        let raw = cell(row, amount);
        let (Some(value), Some(when)) = (num(raw), datetime(cell(row, date))) else {
            ignored += 1;
            continue;
        };
        let cur = currency
            .and_then(|i| fiat(cell(row, i)))
            .or_else(|| currency_in(raw))
            .unwrap_or_else(|| "HUF".into());
        let fee = fee.and_then(|i| num(cell(row, i))).unwrap_or(0.0).abs();
        // Kiutalásnál a díj a kiadást növeli, jóváírásnál a bevételt csökkenti.
        let net = value - fee;
        if net == 0.0 {
            ignored += 1;
            continue;
        }
        if !matched.contains(&name) {
            matched.push(name);
        }
        flows.push(FiatFlow { date: when.date(), currency: cur, amount: net, fee });
    }

    let mut warnings = vec![
        "Banki kivonatból a tőzsdére utalt és onnan érkezett összegeket veszem át. Ez csak közelítés: ha a tőzsdén egyenleg \
         maradt, vagy a tőzsdei exportot is importálod, a kiadás eltérhet vagy duplán számolhat."
            .to_string(),
    ];
    if !matched.is_empty() {
        warnings.push(format!("Felismert tőzsdék: {}.", matched.join(", ")));
    }
    Ok(Parsed { source: "bank", format: "Banki kivonat – tőzsdei utalások", flows, ignored, warnings })
}

#[cfg(test)]
mod tests {
    use super::super::*;

    const REVOLUT: &str = "Type,Product,Started Date,Completed Date,Description,Amount,Fee,Currency,State,Balance
TRANSFER,Current,2026-02-01 10:00:00,2026-02-01 10:01:00,To BINANCE,-500.00,0.00,EUR,COMPLETED,1500.00
CARD_PAYMENT,Current,2026-02-02 12:00:00,2026-02-02 12:00:00,Tesco,-23.50,0.00,EUR,COMPLETED,1476.50
TRANSFER,Current,2026-03-05 09:00:00,2026-03-05 09:00:00,Payment from Payward Ltd,320.00,0.00,EUR,COMPLETED,1796.50
TRANSFER,Current,2026-03-06 09:00:00,,To Bybit,-100.00,0.00,EUR,REVERTED,1796.50
";

    const OTP: &str = "Könyvelés dátuma;Összeg;Pénznem;Partner neve;Közlemény
2026.01.10;-200 000;HUF;Clear Junction Ltd;BINANCE 1234
2026.01.11;-15 000;HUF;MOL Nyrt;tankolás
2026.04.02;150 000;HUF;Coinbase Europe;kifizetés
";

    #[test]
    fn revolut_account_statement() {
        let t = table_from_rows(read_csv(REVOLUT).unwrap()).unwrap();
        assert!(!binance::detect(&t) && !revolut::detect(&t) && super::detect(&t));
        let p = super::parse(&t).unwrap();
        let rows = aggregate(p.source, &p.flows);
        assert_eq!(rows.len(), 2, "{rows:#?}");
        assert_eq!((rows[0].kind.as_str(), rows[0].amount), ("buy", 500.0));
        assert_eq!((rows[1].kind.as_str(), rows[1].amount), ("sell", 320.0));
        assert_eq!(p.ignored, 2);
    }

    #[test]
    fn hungarian_bank_statement() {
        let t = table_from_rows(read_csv(OTP).unwrap()).unwrap();
        assert!(super::detect(&t));
        let p = super::parse(&t).unwrap();
        let rows = aggregate(p.source, &p.flows);
        assert_eq!(rows.len(), 2, "{rows:#?}");
        assert_eq!((rows[0].currency.as_str(), rows[0].kind.as_str(), rows[0].amount), ("HUF", "buy", 200_000.0));
        assert_eq!((rows[1].kind.as_str(), rows[1].amount), ("sell", 150_000.0));
    }
}
