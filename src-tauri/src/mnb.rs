//! MNB hivatalos devizaárfolyamok lekérése az MNB SOAP webszolgáltatásából.

use chrono::NaiveDate;
use std::time::Duration;

// A SOAP végpont csak HTTP-n él (HTTPS-en 404); nyilvános árfolyamadat, nincs benne érzékeny adat.
const URL: &str = "http://www.mnb.hu/arfolyamok.asmx";
const SOAP_ACTION: &str = "http://www.mnb.hu/webservices/MNBArfolyamServiceSoap/GetExchangeRates";

/// Az összes közzétett (nap, árfolyam) pár a [start, end] időszakra. Az árfolyam 1 egységre vonatkozik.
pub fn fetch_range(currency: &str, start: NaiveDate, end: NaiveDate) -> Result<Vec<(NaiveDate, f64)>, String> {
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body><GetExchangeRates xmlns="http://www.mnb.hu/webservices/"><startDate>{start}</startDate><endDate>{end}</endDate><currencyNames>{currency}</currencyNames></GetExchangeRates></soap:Body></soap:Envelope>"#
    );
    let xml = ureq::post(URL)
        .timeout(Duration::from_secs(15))
        .set("Content-Type", "text/xml; charset=utf-8")
        .set("SOAPAction", SOAP_ACTION)
        .send_string(&body)
        .map_err(|e| format!("Nem sikerült elérni az MNB-t: {e}"))?
        .into_string()
        .map_err(|e| format!("Hibás MNB válasz: {e}"))?;
    Ok(parse(&xml))
}

/// A válaszban a tényleges adat escape-elt XML: `<Day date="2026-10-26"><Rate unit="1" curr="EUR">366,95000</Rate></Day>`.
fn parse(xml: &str) -> Vec<(NaiveDate, f64)> {
    let xml = xml.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"");
    let mut out = Vec::new();
    for chunk in xml.split("<Day date=\"").skip(1) {
        let Some(date) = chunk.get(..10).and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()) else {
            continue;
        };
        let Some(start) = chunk.find("<Rate") else { continue };
        let rate = &chunk[start..];
        let unit: f64 = attr(rate, "unit").and_then(|u| u.parse().ok()).unwrap_or(1.0);
        let (Some(gt), Some(end)) = (rate.find('>'), rate.find("</Rate>")) else { continue };
        if let Ok(value) = rate[gt + 1..end].trim().replace(',', ".").parse::<f64>() {
            out.push((date, value / unit));
        }
    }
    out
}

fn attr<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let i = s.find(&key)? + key.len();
    let j = s[i..].find('"')?;
    Some(&s[i..i + j])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_escaped_response() {
        let xml = r#"<GetExchangeRatesResult>&lt;MNBExchangeRates&gt;&lt;Day date="2026-10-26"&gt;&lt;Rate unit="1" curr="EUR"&gt;366,95000&lt;/Rate&gt;&lt;/Day&gt;&lt;Day date="2026-10-22"&gt;&lt;Rate unit="100" curr="JPY"&gt;240,10000&lt;/Rate&gt;&lt;/Day&gt;&lt;/MNBExchangeRates&gt;</GetExchangeRatesResult>"#;
        let days = parse(xml);
        assert_eq!(days.len(), 2);
        assert_eq!(days[0], (NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(), 366.95));
        assert!((days[1].1 - 2.401).abs() < 1e-9);
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    /// `cargo test -- --ignored mnb_live`
    #[test]
    #[ignore]
    fn mnb_live() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap(); // vasárnap
        let days = fetch_range("EUR", d - chrono::Duration::days(14), d).unwrap();
        println!("{days:?}");
        assert!(days.iter().all(|(day, rate)| *day <= d && *rate > 100.0));
    }
}
