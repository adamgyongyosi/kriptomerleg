//! Kraken főkönyv (Ledgers) lekérése csak olvasó API kulccsal. Az eredmény ugyanaz a tábla, mint
//! a „Ledgers” CSV export, így a [`super::kraken`] feldolgozó kezeli.
//!
//! A kulcsot és a titkot nem tároljuk: csak erre az egy lekérésre használjuk.

use super::Table;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use std::time::Duration;

const API: &str = "https://api.kraken.com";
const LEDGERS: &str = "/0/private/Ledgers";
/// A Kraken oldalanként legfeljebb 50 sort ad.
const PAGE: usize = 50;
/// Biztonsági korlát (10 000 sor).
const MAX_PAGES: usize = 200;

/// API-Sign = base64(HMAC-SHA512(útvonal + SHA256(nonce + POST adat), base64-dekódolt titok)).
pub fn sign(path: &str, nonce: u64, postdata: &str, secret_b64: &str) -> Result<String, String> {
    let secret = B64.decode(secret_b64.trim()).map_err(|_| "Hibás API titok (nem base64).".to_string())?;
    let digest = Sha256::digest(format!("{nonce}{postdata}").as_bytes());
    let mut mac = Hmac::<Sha512>::new_from_slice(&secret).map_err(|e| e.to_string())?;
    mac.update(path.as_bytes());
    mac.update(&digest);
    Ok(B64.encode(mac.finalize().into_bytes()))
}

/// Az összes főkönyvi sor `start` (Unix idő) óta, a CSV export oszlopaival.
pub fn fetch_ledgers(key: &str, secret: &str, start: i64) -> Result<Table, String> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut ofs = 0;
    for page in 0..MAX_PAGES {
        if page > 0 {
            // Lekérési korlát: a Ledgers hívás 2 pontba kerül, a keret lassan töltődik vissza.
            std::thread::sleep(Duration::from_millis(2500));
        }
        let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        let post = format!("nonce={nonce}&start={start}&ofs={ofs}");
        let resp = ureq::post(&format!("{API}{LEDGERS}"))
            .timeout(Duration::from_secs(30))
            .set("API-Key", key.trim())
            .set("API-Sign", &sign(LEDGERS, nonce, &post, secret)?)
            .set("Content-Type", "application/x-www-form-urlencoded; charset=utf-8")
            .send_string(&post)
            .map_err(|e| format!("Nem sikerült elérni a Krakent: {e}"))?;
        let body = resp.into_string().map_err(|e| format!("Hibás Kraken válasz: {e}"))?;
        let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("Hibás Kraken válasz: {e}"))?;
        if let Some(errs) = json["error"].as_array().filter(|e| !e.is_empty()) {
            let msg = errs.iter().filter_map(|e| e.as_str()).collect::<Vec<_>>().join(", ");
            return Err(match msg.as_str() {
                m if m.contains("Invalid key") || m.contains("Invalid signature") => "A Kraken elutasította a kulcsot (hibás kulcs vs. titok).".into(),
                m if m.contains("Permission denied") => "A kulcsnak nincs „Query Ledger Entries” jogosultsága.".into(),
                m => format!("Kraken hiba: {m}"),
            });
        }
        let (page_rows, count) = parse_page(&json["result"]);
        let got = page_rows.len();
        rows.extend(page_rows);
        ofs += got;
        if got < PAGE || ofs >= count {
            break;
        }
    }
    let headers = ["txid", "refid", "time", "type", "subtype", "aclass", "asset", "wallet", "amount", "fee", "balance"];
    Ok(Table { headers: headers.iter().map(|h| h.to_string()).collect(), rows })
}

/// Egy válaszoldal sorai (CSV-oszloprendben) és az összes sor száma.
fn parse_page(result: &serde_json::Value) -> (Vec<Vec<String>>, usize) {
    let count = result["count"].as_u64().unwrap_or(0) as usize;
    let Some(ledger) = result["ledger"].as_object() else { return (Vec::new(), count) };
    let s = |v: &serde_json::Value| match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    };
    let rows = ledger
        .iter()
        .map(|(id, e)| {
            let secs = e["time"].as_f64().unwrap_or(0.0) as i64;
            let time = chrono::DateTime::from_timestamp(secs, 0).map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
            vec![
                id.clone(),
                s(&e["refid"]),
                time,
                s(&e["type"]),
                s(&e["subtype"]),
                s(&e["aclass"]),
                s(&e["asset"]),
                s(&e["wallet"]),
                s(&e["amount"]),
                s(&e["fee"]),
                s(&e["balance"]),
            ]
        })
        .collect();
    (rows, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Kraken dokumentáció aláírás-példája.
    #[test]
    fn signature_matches_kraken_docs() {
        let secret = "kQH5HW/8p1uGOVjbgWA7FunAmGO8lsSUXNsu3eow76sz84Q18fWxnyRzBHCd3pd5nE9qa99HAZtuZuj6F1huXg==";
        let post = "nonce=1616492376594&ordertype=limit&pair=XBTUSD&price=37500&type=buy&volume=1.25";
        assert_eq!(
            sign("/0/private/AddOrder", 1616492376594, post, secret).unwrap(),
            "4/dpxb3iT4tp/ZCVEwSnEsLxx0bqyhLpdfOpc6fn7OR8+UClSV5n9E6aSS8MPtnRfp32bAb0nmbRn6H8ndwLUQ=="
        );
    }

    #[test]
    fn page_becomes_ledger_table() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"ledger":{
                "L1":{"refid":"T1","time":1770714600.1,"type":"trade","subtype":"","aclass":"currency","asset":"ZEUR","amount":"-500.0000","fee":"1.3000","balance":"498.7"},
                "L2":{"refid":"T1","time":1770714600.1,"type":"trade","subtype":"","aclass":"currency","asset":"XXBT","amount":"0.006","fee":"0","balance":"0.006"}
            },"count":2}"#,
        )
        .unwrap();
        let (rows, count) = parse_page(&json);
        assert_eq!((rows.len(), count), (2, 2));
        let t = Table {
            headers: ["txid", "refid", "time", "type", "subtype", "aclass", "asset", "wallet", "amount", "fee", "balance"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            rows,
        };
        assert!(super::super::kraken::detect(&t));
        let p = super::super::kraken::parse(&t).unwrap();
        let agg = super::super::aggregate(p.source, &p.flows);
        assert_eq!((agg.len(), agg[0].amount), (1, 501.3));
    }
}
