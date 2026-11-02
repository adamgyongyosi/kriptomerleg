//! Bybit fiat ↔ kripto ügyletek lekérése a V5 API-n.
//!
//! **Csak olvasó (read-only) API kulcs kell**: a Bybit „API Management” oldalán „Read-Only”
//! kulcsot hozz létre. Ha a kulcs írási jogot is ad, figyelmeztetünk. A kulcsot és a titkot nem
//! tároljuk: csak erre az egy lekérésre használjuk.
//!
//! Lekért adatok (hivatalos dokumentáció: <https://bybit-exchange.github.io/docs/v5/intro>):
//! - `GET /v5/user/query-api` – a kulcs ellenőrzése (bármely jogosultsággal hívható): csak olvasó-e,
//!   egységes (UTA) vagy klasszikus fiók;
//! - `GET /v5/market/instruments-info?category=spot` (nyilvános) – a párok alap- és jegyzési
//!   devizája (BTCEUR → BTC / EUR);
//! - `GET /v5/execution/list?category=spot` – spot ügyletek. Egy kérés legfeljebb 7 napot fed le,
//!   és csak az utolsó 2 év kérdezhető le; 7 napos ablakonként `cursor`-ral lapozunk;
//! - `GET /v5/asset/exchange/query-convert-history` – Convert (2025. szeptember 10-től a weben
//!   végzett is), oldalszámmal lapozva;
//! - `GET /v5/asset/exchange/order-record` – régebbi „coin exchange” tételek, `cursor`-ral;
//! - `GET /v5/fiat/query-trade-history` – fiat Convert (ahol a régió támogatja).
//!
//! A három Convert-forrás átfedhet, ezért azonosító és tartalom (devizák, összegek, időpont) szerint
//! kiszűrjük az ismétlődést. Ha valamelyik Convert-végpont a fiókra nem érhető el, csak
//! figyelmeztetünk.
//!
//! A hálózati rész ([`fetch`]) és a válaszok feldolgozása (`parse_*`) külön van, hogy a
//! feldolgozás a dokumentáció példaválaszaival tesztelhető legyen.

use super::{FIAT, FiatFlow, Parsed, budapest_date, fiat};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A globális Bybit; az EGT-s (bybit.eu) fiókok kulcsa a saját címükön érvényes.
const HOSTS: &[&str] = &["https://api.bybit.com", "https://api.bybit.eu"];
const RECV_WINDOW: u32 = 10_000;
const DAY_MS: i64 = 86_400_000;
/// Az execution/list egy kérése legfeljebb 7 napot fedhet le.
const WEEK_MS: i64 = 7 * DAY_MS;
/// Az execution/list csak az utolsó 2 évet adja; egy nap ráhagyással.
const HISTORY_MS: i64 = 729 * DAY_MS;
const EXEC_LIMIT: usize = 100;
/// Biztonsági korlát 7 napos ablakonként (10 000 ügylet).
const MAX_EXEC_PAGES: usize = 100;
const CONVERT_LIMIT: usize = 100;
const ORDER_RECORD_LIMIT: usize = 50;
const MAX_CONVERT_PAGES: usize = 200;
/// Az összes kérés biztonsági korlátja.
const MAX_REQUESTS: usize = 5000;
/// Két Convert-tétel azonos, ha minden adatuk egyezik és ennyin belül történtek.
const SAME_CONVERT_MS: i64 = 120_000;

/// Hiba: a végzetes leállítja a lekérést, az enyhe csak figyelmeztetés; a `BadKey` után a
/// következő címmel (bybit.eu) még megpróbáljuk.
#[derive(Debug)]
enum Fail {
    Fatal(String),
    Soft(String),
    BadKey,
}

const BAD_KEY: &str = "A Bybit elutasította a kulcsot (érvénytelen API kulcs). Ellenőrizd, hogy éles (nem testnet/demo) kulcsot adtál-e meg. Bybit EU (bybit.eu) fióknál az API csak korlátozottan érhető el – ilyenkor fájlexportból importálj.";

impl From<Fail> for String {
    fn from(f: Fail) -> String {
        match f {
            Fail::Fatal(m) | Fail::Soft(m) => m,
            Fail::BadKey => BAD_KEY.into(),
        }
    }
}

/// Bájtok kisbetűs hexa alakja.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Az aláírandó szöveg GET kérésnél: timestamp + API kulcs + recv_window + lekérdezési szöveg.
pub fn payload(timestamp: i64, key: &str, recv_window: u32, query: &str) -> String {
    format!("{timestamp}{key}{recv_window}{query}")
}

/// X-BAPI-SIGN = hex(HMAC-SHA256(aláírandó szöveg, titok)).
pub fn sign(payload: &str, secret: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.trim().as_bytes()).expect("a HMAC bármilyen kulcshosszt elfogad");
    mac.update(payload.as_bytes());
    hex(&mac.finalize().into_bytes())
}

fn local_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn pause(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/* ───────────── Hálózat ───────────── */

struct Client<'a> {
    host: &'static str,
    key: &'a str,
    secret: &'a str,
    /// A Bybit szerveridő és a helyi óra különbsége (ms).
    offset_ms: i64,
    requests: usize,
}

impl Client<'_> {
    fn now_ms(&self) -> i64 {
        local_ms() + self.offset_ms
    }

    /// GET kérés; sikeres válasznál (`retCode` = 0) a teljes JSON.
    fn get(&mut self, path: &str, query: &str, signed: bool) -> Result<Value, Fail> {
        let url = if query.is_empty() { format!("{}{path}", self.host) } else { format!("{}{path}?{query}", self.host) };
        for attempt in 0..4 {
            self.requests += 1;
            if self.requests > MAX_REQUESTS {
                return Err(Fail::Fatal("Túl sok Bybit lekérés kellene; adj meg későbbi kezdődátumot.".into()));
            }
            let mut req = ureq::get(&url).timeout(Duration::from_secs(30));
            if signed {
                let ts = self.now_ms();
                let key = self.key.trim();
                req = req
                    .set("X-BAPI-API-KEY", key)
                    .set("X-BAPI-TIMESTAMP", &ts.to_string())
                    .set("X-BAPI-RECV-WINDOW", &RECV_WINDOW.to_string())
                    .set("X-BAPI-SIGN", &sign(&payload(ts, key, RECV_WINDOW, query), self.secret));
            }
            let json: Value = match req.call() {
                Ok(resp) => {
                    // Végpontonkénti keret: ha elfogyott, megvárjuk a visszatöltődést.
                    let left = resp.header("X-Bapi-Limit-Status").and_then(|v| v.trim().parse::<i64>().ok());
                    let reset = resp.header("X-Bapi-Limit-Reset-Timestamp").and_then(|v| v.trim().parse::<i64>().ok());
                    let json = serde_json::from_reader(resp.into_reader()).map_err(|e| Fail::Fatal(format!("Hibás Bybit válasz: {e}")))?;
                    if let (Some(0), Some(reset)) = (left, reset) {
                        pause((reset - local_ms()).clamp(100, 5000) as u64);
                    }
                    json
                }
                Err(ureq::Error::Status(403, _)) => {
                    return Err(Fail::Fatal(
                        "A Bybit elutasította a kérést (HTTP 403): túl sok kérés erről az IP-címről, vagy a Bybit erről a helyről nem érhető el. Várj legalább 10 percet, majd próbáld újra.".into(),
                    ));
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let body = resp.into_string().unwrap_or_default();
                    serde_json::from_str(&body).map_err(|_| Fail::Soft(format!("Bybit hiba (HTTP {code})")))?
                }
                // Az URL-t (aláírt lekérdezéssel) nem írjuk ki, csak a hiba fajtáját.
                Err(e) => return Err(Fail::Fatal(format!("Nem sikerült elérni a Bybitet: {}", e.kind()))),
            };
            let code = json["retCode"].as_i64().unwrap_or(-1);
            if code == 0 {
                return Ok(json);
            }
            if code == 10006 && attempt < 3 {
                // Túl sok kérés: rövid várakozás után újra.
                pause(2000);
                continue;
            }
            return Err(api_error(code, json["retMsg"].as_str().unwrap_or("")));
        }
        Err(Fail::Fatal("A Bybit túl sok kérést jelez. Próbáld újra néhány perc múlva.".into()))
    }
}

/// A Bybit hibakódja (`retCode`) érthető magyar üzenetként.
fn api_error(code: i64, msg: &str) -> Fail {
    match code {
        10003 => Fail::BadKey,
        10004 => Fail::Fatal("A Bybit elutasította az aláírást: hibás a titkos kulcs (API Secret).".into()),
        10005 => Fail::Soft(format!("a kulcsnak nincs ehhez olvasási jogosultsága ({msg})")),
        10002 => Fail::Fatal(
            "A számítógép órája eltér a Bybit szerveridejétől. Állítsd az órát automatikus időszinkronra, majd próbáld újra.".into(),
        ),
        10010 => Fail::Fatal("A kulcs IP-címhez van kötve, és ez a gép más címről fut. Engedélyezd az IP-t, vagy hozz létre új kulcsot.".into()),
        33004 => Fail::Fatal("A Bybit API kulcs lejárt; hozz létre újat.".into()),
        _ => Fail::Soft(format!("Bybit hiba ({code}): {msg}")),
    }
}

/* ───────────── Lekérés ───────────── */

/// Az összes fiat ↔ kripto ügylet `start_ms` (Unix ms) óta.
pub fn fetch(key: &str, secret: &str, start_ms: i64) -> Result<Parsed, String> {
    if key.trim().is_empty() || secret.trim().is_empty() {
        return Err("Add meg a Bybit API kulcsot és a titkos kulcsot.".into());
    }
    let mut c = Client { host: HOSTS[0], key, secret, offset_ms: 0, requests: 0 };
    run(&mut c, start_ms).map_err(String::from)
}

fn run(c: &mut Client, start_ms: i64) -> Result<Parsed, Fail> {
    let mut acc = Acc::default();

    // Szerveridő (a közös válaszmező `time`, ms): ehhez igazítjuk az aláírt kérések időbélyegét.
    if let Some(server) = c.get("/v5/market/time", "", false)?["time"].as_i64() {
        c.offset_ms = server - local_ms();
    }

    // A kulcs ellenőrzése; érvénytelen kulcsnál az EGT-s címmel is megpróbáljuk.
    let mut info = None;
    for host in HOSTS {
        c.host = host;
        match c.get("/v5/user/query-api", "", true) {
            Ok(v) => {
                info = Some(v);
                break;
            }
            Err(Fail::BadKey) => continue,
            Err(Fail::Soft(m)) => {
                acc.failed.push(format!("kulcs-adatok: {m}"));
                info = Some(Value::Null);
                break;
            }
            Err(f) => return Err(f),
        }
    }
    let Some(info) = info else { return Err(Fail::BadKey) };
    acc.warnings.extend(check_key(&info["result"]));
    let now = c.now_ms();

    // 1. Spot ügyletek (az utolsó 2 év, 7 napos ablakokban).
    let pairs = match c.get("/v5/market/instruments-info", "category=spot", false) {
        Ok(v) => spot_pairs(&v["result"]),
        Err(Fail::Soft(m)) => {
            acc.failed.push(format!("párok listája: {m}"));
            HashMap::new()
        }
        Err(f) => return Err(f),
    };
    let earliest = now - HISTORY_MS;
    if start_ms < earliest {
        acc.warnings.push(format!(
            "A Bybit API csak az utolsó 2 év spot ügyleteit adja vissza; a {} előtti spot ügyleteket a Bybit fájlexportjából importáld.",
            date_of(earliest).map(|d| d.to_string()).unwrap_or_default()
        ));
    }
    let mut seen = HashSet::new();
    'windows: for (s, e) in windows(start_ms.max(earliest), now, WEEK_MS) {
        let mut cursor = String::new();
        for page in 0..MAX_EXEC_PAGES {
            pause(120);
            let mut q = format!("category=spot&startTime={s}&endTime={e}&limit={EXEC_LIMIT}");
            if !cursor.is_empty() {
                // A kurzort változatlanul (ahogy a válaszban jött) küldjük és írjuk alá.
                q.push_str(&format!("&cursor={cursor}"));
            }
            match c.get("/v5/execution/list", &q, true) {
                Ok(v) => {
                    let list = v["result"]["list"].as_array().map(Vec::as_slice).unwrap_or(&[]);
                    parse_executions(list, &pairs, start_ms, &mut seen, &mut acc);
                    cursor = v["result"]["nextPageCursor"].as_str().unwrap_or("").to_string();
                    if cursor.is_empty() || list.is_empty() {
                        break;
                    }
                    if page + 1 == MAX_EXEC_PAGES {
                        acc.truncated.push(format!("spot ügyletek {}", iso(s)));
                    }
                }
                Err(Fail::Soft(m)) => {
                    acc.failed.push(format!("spot ügyletek: {m}"));
                    break 'windows;
                }
                Err(f) => return Err(f),
            }
        }
    }

    // 2. Convert, három forrásból.
    let mut converts = Vec::new();
    for page in 1..=MAX_CONVERT_PAGES {
        pause(200);
        match c.get("/v5/asset/exchange/query-convert-history", &format!("index={page}&limit={CONVERT_LIMIT}"), true) {
            Ok(v) => {
                let got = parse_convert_history(&v["result"]);
                let n = got.len();
                converts.extend(got);
                if n < CONVERT_LIMIT {
                    break;
                }
                if page == MAX_CONVERT_PAGES {
                    acc.truncated.push("Convert".into());
                }
            }
            Err(Fail::Soft(m)) => {
                acc.failed.push(format!("Convert előzmények: {m}"));
                break;
            }
            Err(f) => return Err(f),
        }
    }
    let mut cursor = String::new();
    for page in 0..MAX_CONVERT_PAGES {
        pause(150);
        let mut q = format!("limit={ORDER_RECORD_LIMIT}");
        if !cursor.is_empty() {
            q = format!("cursor={cursor}&{q}");
        }
        match c.get("/v5/asset/exchange/order-record", &q, true) {
            Ok(v) => {
                let got = parse_order_record(&v["result"]);
                let n = got.len();
                converts.extend(got);
                cursor = v["result"]["nextPageCursor"].as_str().unwrap_or("").to_string();
                if cursor.is_empty() || n == 0 {
                    break;
                }
                // Ha az oldal minden tétele a kezdődátum előtti, a régebbiek sem kellenek.
                if converts[converts.len() - n..].iter().all(|r| r.time_ms < start_ms) {
                    break;
                }
                if page + 1 == MAX_CONVERT_PAGES {
                    acc.truncated.push("coin exchange".into());
                }
            }
            Err(Fail::Soft(m)) => {
                acc.failed.push(format!("coin exchange előzmények: {m}"));
                break;
            }
            Err(f) => return Err(f),
        }
    }
    for page in 1..=MAX_CONVERT_PAGES {
        pause(200);
        let q = format!("index={page}&limit={CONVERT_LIMIT}&startTime={start_ms}&endTime={now}");
        match c.get("/v5/fiat/query-trade-history", &q, true) {
            Ok(v) => {
                let got = parse_fiat_history(&v["result"]);
                let n = got.len();
                converts.extend(got);
                if n < CONVERT_LIMIT {
                    break;
                }
            }
            // A fiat Convert nem minden régióban érhető el: ezt külön nem jelezzük hibaként.
            Err(Fail::Soft(_)) => break,
            Err(f) => return Err(f),
        }
    }
    convert_flows(converts, start_ms, &mut acc);

    Ok(acc.finish())
}

/// [start, end] felosztása legfeljebb `size` hosszú, egymást nem fedő ablakokra.
fn windows(start: i64, end: i64, size: i64) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    let mut s = start;
    while s <= end {
        let e = (s + size - 1).min(end);
        out.push((s, e));
        s = e + 1;
    }
    out
}

/* ───────────── Feldolgozás ───────────── */

#[derive(Default)]
struct Acc {
    flows: Vec<FiatFlow>,
    ignored: usize,
    warnings: Vec<String>,
    /// Díj harmadik eszközben, aminek nincs árfolyama az ügyletben.
    fee_unknown: usize,
    /// Ügyletek külön adó/díj adattal (`extraFees`), amit nem számoltunk bele.
    extra_fees: usize,
    /// Sikertelen (nem végzetes) lekérések.
    failed: Vec<String>,
    /// Biztonsági korlát miatt csonka lekérések.
    truncated: Vec<String>,
}

impl Acc {
    fn finish(mut self) -> Parsed {
        if self.fee_unknown > 0 {
            self.warnings.push(format!(
                "{} ügyletnél a díj harmadik eszközben volt; ennek fiat értéke az ügyletből nem számolható, ezért 0-nak vettem. Ha számít, rögzítsd kézzel.",
                self.fee_unknown
            ));
        }
        if self.extra_fees > 0 {
            self.warnings.push(format!(
                "{} ügyletnél a Bybit külön adót/díjat is jelzett (extraFees); ezt nem számoltam bele, a nyomkövető szövegben megtalálod.",
                self.extra_fees
            ));
        }
        if !self.failed.is_empty() {
            self.warnings.push(format!(
                "Néhány lekérés nem sikerült, ezek ügyletei hiányozhatnak: {}. Ezeket fájlexportból importáld.",
                self.failed.join("; ")
            ));
        }
        if !self.truncated.is_empty() {
            self.warnings.push(format!(
                "Túl sok tétel miatt a lekérés csonka lehet: {}. A hiányzó részt fájlexportból importáld.",
                self.truncated.join(", ")
            ));
        }
        Parsed { source: "bybit", format: "Bybit – API", flows: self.flows, ignored: self.ignored, warnings: self.warnings }
    }

    fn push(&mut self, ms: i64, currency: String, amount: f64, fee: f64, raw: String) {
        match date_of(ms) {
            Some(date) => self.flows.push(FiatFlow { date, currency, amount, fee, raw, ..Default::default() }),
            None => self.ignored += 1,
        }
    }
}

/// Szám a JSON-ból (a Bybit szövegként adja: `"0.071409"`).
fn number(v: &Value) -> Option<f64> {
    match v {
        Value::String(s) => s.trim().parse().ok(),
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

/// Egész a JSON-ból (az időbélyegek szövegként jönnek: `"1672282722429"`).
fn integer(v: &Value) -> Option<i64> {
    match v {
        Value::String(s) => s.trim().parse().ok(),
        Value::Number(n) => n.as_i64(),
        _ => None,
    }
}

/// Egy JSON érték nyers szövege a nyomkövetéshez.
fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn utc(ms: i64) -> Option<chrono::NaiveDateTime> {
    chrono::DateTime::from_timestamp_millis(ms).map(|d| d.naive_utc())
}

fn date_of(ms: i64) -> Option<chrono::NaiveDate> {
    utc(ms).map(budapest_date)
}

fn iso(ms: i64) -> String {
    utc(ms).map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_else(|| ms.to_string())
}

/// A kulcs adatai (`/v5/user/query-api`): írási jog és fióktípus.
fn check_key(r: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if r["readOnly"].as_i64() == Some(0) {
        out.push("A megadott kulcs írási jogot is ad. Biztonsági okból hozz létre „Read-Only” kulcsot.".into());
    }
    if r["uta"].as_i64() == Some(0) {
        out.push(
            "Klasszikus (nem egységes, UTA előtti) Bybit fiók: a régi spot ügyletek az API-n hiányosak lehetnek. Ha valami hiányzik, a Bybit fájlexportjából importáld."
                .into(),
        );
    }
    out
}

/// Spot párok: szimbólum → (alapdeviza, jegyzési deviza), az `instruments-info` válaszból.
fn spot_pairs(result: &Value) -> HashMap<String, (String, String)> {
    result["list"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|i| {
            Some((i["symbol"].as_str()?.to_string(), (i["baseCoin"].as_str()?.to_uppercase(), i["quoteCoin"].as_str()?.to_uppercase())))
        })
        .collect()
}

/// Kivezetett (a listában már nem szereplő) párnál: "BTCEUR" → ("BTC", "EUR"), ha a vége fiat.
fn split_by_suffix(symbol: &str) -> Option<(String, String)> {
    let s = symbol.to_uppercase();
    FIAT.iter()
        .find(|f| s.len() > f.len() && s.ends_with(*f))
        .map(|f| (s[..s.len() - f.len()].to_string(), f.to_string()))
}

/// `execution/list` (spot) tételei → FiatFlow-k. Az ismétlődőket `execId` alapján kihagyjuk.
fn parse_executions(
    list: &[Value],
    pairs: &HashMap<String, (String, String)>,
    start_ms: i64,
    seen: &mut HashSet<String>,
    acc: &mut Acc,
) {
    for x in list {
        let id = text(&x["execId"]);
        if !id.is_empty() && !seen.insert(id.clone()) {
            continue;
        }
        let symbol = x["symbol"].as_str().unwrap_or("");
        let Some((base, quote)) = pairs.get(symbol).cloned().or_else(|| split_by_suffix(symbol)) else {
            acc.ignored += 1;
            continue;
        };
        let (currency, fiat_is_quote) = match (fiat(&base), fiat(&quote)) {
            (None, Some(q)) => (q, true),
            (Some(b), None) => (b, false),
            _ => {
                acc.ignored += 1;
                continue;
            }
        };
        // Spotnál a végrehajtás típusa `Trade` (klasszikus fióknál `UNKNOWN` vagy üres is lehet).
        if !matches!(x["execType"].as_str().unwrap_or(""), "Trade" | "UNKNOWN" | "") {
            acc.ignored += 1;
            continue;
        }
        let buy = match x["side"].as_str().unwrap_or("") {
            "Buy" => true,
            "Sell" => false,
            _ => {
                acc.ignored += 1;
                continue;
            }
        };
        let (Some(time), Some(qty), Some(price)) = (integer(&x["execTime"]), number(&x["execQty"]), number(&x["execPrice"])) else {
            acc.ignored += 1;
            continue;
        };
        if time < start_ms {
            continue;
        }
        let value = number(&x["execValue"]).unwrap_or(qty * price).abs();
        // A fiat oldal bruttó összege, és hogy fiat ment-e ki.
        // BTCEUR: vételnél EUR megy ki; EURUSDT: vételnél EUR jön be (kriptót adtunk el érte).
        let (gross, fiat_out) = if fiat_is_quote { (value, buy) } else { (qty.abs(), !buy) };
        let raw_fee = number(&x["execFee"]).unwrap_or(0.0);
        // A díj devizája; ha üres, a dokumentáció „Spot fee currency instruction” szabálya szerint
        // (pozitív díjnál) vételkor az alapdeviza, eladáskor a jegyzési deviza.
        let fee_ccy = match x["feeCurrency"].as_str().unwrap_or("").to_uppercase() {
            c if c.is_empty() => if buy { base.clone() } else { quote.clone() },
            c => c,
        };
        let fee = if raw_fee == 0.0 {
            0.0
        } else if fiat(&fee_ccy).as_deref() == Some(currency.as_str()) {
            raw_fee
        } else if fee_ccy == base && fiat_is_quote {
            // A díj a megvett kriptóban: az ügylet árfolyamán átváltva.
            raw_fee * price
        } else if fee_ccy == quote && !fiat_is_quote && price > 0.0 {
            raw_fee / price
        } else {
            acc.fee_unknown += 1;
            0.0
        };
        let extra = text(&x["extraFees"]);
        if !matches!(extra.trim(), "" | "[]" | "{}") {
            acc.extra_fees += 1;
        }
        let amount = if fiat_out { -(gross + fee) } else { gross - fee };
        let mut raw = format!(
            "execution {symbol} execId={id} {} {} execQty={} execValue={} execPrice={} execFee={} {fee_ccy}",
            iso(time),
            if buy { "buy" } else { "sell" },
            text(&x["execQty"]),
            text(&x["execValue"]),
            text(&x["execPrice"]),
            text(&x["execFee"]),
        );
        if !extra.trim().is_empty() {
            raw.push_str(&format!(" extraFees={extra}"));
        }
        // Negatív díj (maker-visszatérítés) a nettó összegben benne van, a díj oszlopban nem.
        acc.push(time, currency, amount, fee.max(0.0), raw);
    }
}

/// Egy Convert-tétel, bármelyik Convert-végpontból.
#[derive(Debug, Clone, PartialEq)]
struct ConvertRec {
    source: &'static str,
    id: String,
    time_ms: i64,
    ok: bool,
    from: String,
    from_amount: f64,
    to: String,
    to_amount: f64,
    note: String,
}

fn convert_rec(source: &'static str, id: String, time_ms: Option<i64>, ok: bool, r: &Value, note: String) -> Option<ConvertRec> {
    Some(ConvertRec {
        source,
        id,
        time_ms: time_ms?,
        ok,
        from: r["fromCoin"].as_str()?.to_uppercase(),
        from_amount: number(&r["fromAmount"])?.abs(),
        to: r["toCoin"].as_str()?.to_uppercase(),
        to_amount: number(&r["toAmount"])?.abs(),
        note,
    })
}

/// `query-convert-history`: `result.list`, `createdAt` ms, `exchangeStatus` = success.
fn parse_convert_history(result: &Value) -> Vec<ConvertRec> {
    result["list"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|r| {
            let ok = r["exchangeStatus"].as_str() == Some("success");
            convert_rec("query-convert-history", text(&r["exchangeTxId"]), integer(&r["createdAt"]), ok, r, text(&r["accountType"]))
        })
        .collect()
}

/// `exchange/order-record`: `result.orderBody`, `createdTime` másodpercben, státusz nincs.
fn parse_order_record(result: &Value) -> Vec<ConvertRec> {
    result["orderBody"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|r| {
            let ms = integer(&r["createdTime"]).map(|s| s * 1000);
            convert_rec("order-record", text(&r["exchangeTxId"]), ms, true, r, String::new())
        })
        .collect()
}

/// `fiat/query-trade-history`: a `result` maga a tömb, `createdAt` ms, `status` = success.
fn parse_fiat_history(result: &Value) -> Vec<ConvertRec> {
    result
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|r| {
            let ok = r["status"].as_str() == Some("success");
            convert_rec("fiat/query-trade-history", text(&r["tradeNo"]), integer(&r["createdAt"]), ok, r, String::new())
        })
        .collect()
}

/// Convert-tételek → FiatFlow-k, a források közti ismétlődés kiszűrésével. Díj külön nincs (az
/// árfolyamban van).
fn convert_flows(recs: Vec<ConvertRec>, start_ms: i64, acc: &mut Acc) {
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0);
    let mut kept: Vec<ConvertRec> = Vec::new();
    for r in recs {
        if r.time_ms < start_ms {
            continue;
        }
        let duplicate = kept.iter().any(|k| {
            (!r.id.is_empty() && k.id == r.id)
                || (k.from == r.from
                    && k.to == r.to
                    && close(k.from_amount, r.from_amount)
                    && close(k.to_amount, r.to_amount)
                    && (k.time_ms - r.time_ms).abs() <= SAME_CONVERT_MS)
        });
        if duplicate {
            continue;
        }
        kept.push(r);
    }
    for r in kept {
        if !r.ok {
            acc.ignored += 1;
            continue;
        }
        let note = if r.note.is_empty() { String::new() } else { format!(" ({})", r.note) };
        let raw = format!("{} id={} {} {} {} -> {} {}{note}", r.source, r.id, iso(r.time_ms), r.from, r.from_amount, r.to, r.to_amount);
        match (fiat(&r.from), fiat(&r.to)) {
            (Some(c), None) => acc.push(r.time_ms, c, -r.from_amount, 0.0, raw),
            (None, Some(c)) => acc.push(r.time_ms, c, r.to_amount, 0.0, raw),
            _ => acc.ignored += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// A dokumentáció GET-példájának aláírandó szövege („Integration Guidance” → Authentication).
    /// A dokumentáció a példa titkát nem közli, ezért az aláírást a HMAC-SHA256 szabványos
    /// tesztvektorával (RFC 4231, 2. eset) ellenőrizzük.
    #[test]
    fn signature() {
        assert_eq!(
            payload(1658384314791, "XXXXXXXXXX", 5000, "category=option&symbol=BTC-29JUL22-25000-C"),
            "1658384314791XXXXXXXXXX5000category=option&symbol=BTC-29JUL22-25000-C"
        );
        assert_eq!(sign("what do ya want for nothing?", "Jefe"), "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    }

    #[test]
    fn time_windows() {
        let w = windows(0, 3 * WEEK_MS, WEEK_MS);
        assert_eq!(w.len(), 4);
        assert!(w.iter().all(|(s, e)| e - s < WEEK_MS));
        assert_eq!(w[1].0, WEEK_MS);
    }

    fn pairs() -> HashMap<String, (String, String)> {
        let v: Value = serde_json::from_str(
            r#"{"category":"spot","list":[
                {"symbol":"BTCEUR","baseCoin":"BTC","quoteCoin":"EUR","status":"Trading"},
                {"symbol":"USDTEUR","baseCoin":"USDT","quoteCoin":"EUR","status":"Trading"},
                {"symbol":"EURUSDT","baseCoin":"EUR","quoteCoin":"USDT","status":"Trading"},
                {"symbol":"BTCUSDT","baseCoin":"BTC","quoteCoin":"USDT","status":"Trading"}
            ]}"#,
        )
        .unwrap();
        spot_pairs(&v)
    }

    /// Az `execution/list` dokumentációs példája (ETHPERP, linear) nem fiat pár: kimarad.
    #[test]
    fn doc_execution_is_ignored() {
        let v: Value = serde_json::from_str(
            r#"{"retCode":0,"retMsg":"OK","result":{"nextPageCursor":"132766%3A2%2C132766%3A2","category":"linear","list":[
                {"symbol":"ETHPERP","orderType":"Market","underlyingPrice":"","orderLinkId":"","side":"Buy","indexPrice":"","orderId":"8c065341-7b52-4ca9-ac2c-37e31ac55c94","stopOrderType":"UNKNOWN","leavesQty":"0","execTime":"1672282722429","feeCurrency":"","isMaker":false,"execFee":"0.071409","feeRate":"0.0006","execId":"e0cbe81d-0f18-5866-9415-cf319b5dab3b","tradeIv":"","blockTradeId":"","markPrice":"1183.54","execPrice":"1190.15","markIv":"","orderQty":"0.1","orderPrice":"1236.9","execValue":"119.015","execType":"Trade","execQty":"0.1","closedSize":"","extraFees":"","seq":4688002127}
            ]},"retExtInfo":{},"time":1672283754510}"#,
        )
        .unwrap();
        let mut acc = Acc::default();
        parse_executions(v["result"]["list"].as_array().unwrap(), &pairs(), 0, &mut HashSet::new(), &mut acc);
        assert_eq!((acc.flows.len(), acc.ignored), (0, 1));
    }

    /// Spot ügyletek a dokumentáció mezőneveivel: vétel (díj BTC-ben), eladás (díj EUR-ban),
    /// USDT pár (kimarad), ismétlődő execId (kimarad), EURUSDT eladás, üres díjdeviza.
    #[test]
    fn spot_executions() {
        let list: Value = serde_json::from_str(
            r#"[
            {"symbol":"BTCEUR","side":"Buy","orderType":"Market","execId":"e1","execPrice":"60000","execQty":"0.01","execValue":"600","execFee":"0.00001","feeCurrency":"BTC","execType":"Trade","execTime":"1775030400000","isMaker":false,"extraFees":""},
            {"symbol":"BTCEUR","side":"Sell","orderType":"Limit","execId":"e2","execPrice":"70000","execQty":"0.005","execValue":"350","execFee":"0.35","feeCurrency":"EUR","execType":"Trade","execTime":"1777622400000","isMaker":true,"extraFees":""},
            {"symbol":"BTCUSDT","side":"Buy","orderType":"Market","execId":"e3","execPrice":"65000","execQty":"0.0154","execValue":"1001","execFee":"0.00001","feeCurrency":"BTC","execType":"Trade","execTime":"1777708800000","isMaker":false,"extraFees":""},
            {"symbol":"BTCEUR","side":"Buy","orderType":"Market","execId":"e1","execPrice":"60000","execQty":"0.01","execValue":"600","execFee":"0.00001","feeCurrency":"BTC","execType":"Trade","execTime":"1775030400000","isMaker":false,"extraFees":""},
            {"symbol":"EURUSDT","side":"Sell","orderType":"Market","execId":"e4","execPrice":"1.1","execQty":"100","execValue":"110","execFee":"0.11","feeCurrency":"USDT","execType":"Trade","execTime":"1777708800000","isMaker":false,"extraFees":""},
            {"symbol":"BTCEUR","side":"Sell","orderType":"Market","execId":"e5","execPrice":"70000","execQty":"0.001","execValue":"70","execFee":"0.07","feeCurrency":"","execType":"Trade","execTime":"1777708800000","isMaker":false,"extraFees":""}
        ]"#,
        )
        .unwrap();
        let mut acc = Acc::default();
        parse_executions(list.as_array().unwrap(), &pairs(), 0, &mut HashSet::new(), &mut acc);
        assert_eq!((acc.flows.len(), acc.ignored), (4, 1));
        let buy = &acc.flows[0];
        assert_eq!((buy.date.to_string(), buy.currency.as_str()), ("2026-04-01".into(), "EUR"));
        assert!(close(buy.amount, -600.6) && close(buy.fee, 0.6));
        assert!(buy.raw.starts_with("execution BTCEUR execId=e1 2026-04-01T08:00:00Z buy execQty=0.01"), "{}", buy.raw);
        assert!(close(acc.flows[1].amount, 349.65) && close(acc.flows[1].fee, 0.35));
        // EURUSDT eladás: 100 EUR ment ki, a 0.11 USDT díj 0.1 EUR.
        assert!(close(acc.flows[2].amount, -100.1) && close(acc.flows[2].fee, 0.1));
        // Üres díjdeviza eladásnál: a jegyzési deviza (EUR).
        assert!(close(acc.flows[3].amount, 69.93));
    }

    /// Kivezetett pár (nincs az instruments listában): a végződés alapján ismerjük fel.
    #[test]
    fn delisted_pair_by_suffix() {
        assert_eq!(split_by_suffix("ETHPLN"), Some(("ETH".into(), "PLN".into())));
        assert_eq!(split_by_suffix("BTCUSDT"), None);
    }

    /// A három Convert-végpont dokumentációs példái (mind kripto–kripto vagy nem támogatott fiat),
    /// plusz fiat oldalú tételek és a források közti ismétlődés.
    #[test]
    fn converts() {
        let history: Value = serde_json::from_str(
            r#"{"retCode":0,"retMsg":"ok","result":{"list":[
                {"accountType":"eb_convert_funding","exchangeTxId":"10100108106409343501030232064","userId":"XXXXX","fromCoin":"ETH","fromCoinType":"crypto","fromAmount":"0.1","toCoin":"BTC","toCoinType":"crypto","toAmount":"0.00534882723991","exchangeStatus":"success","extInfo":{"paramType":"opFrom","paramValue":"broker-id-001"},"convertRate":"0.0534882723991","createdAt":"1720071899995"},
                {"accountType":"eb_convert_uta","exchangeTxId":"23070eb_convert_uta408933875189391360","userId":"XXXXX","fromCoin":"BTC","fromCoinType":"crypto","fromAmount":"0.1","toCoin":"ETH","toCoinType":"crypto","toAmount":"1.773938248611074","exchangeStatus":"success","extInfo":{},"convertRate":"17.73938248611074","createdAt":"1719974243256"},
                {"accountType":"funding_fiat","exchangeTxId":"f1","userId":"XXXXX","fromCoin":"EUR","fromCoinType":"fiat","fromAmount":"200","toCoin":"BTC","toCoinType":"crypto","toAmount":"0.003","exchangeStatus":"success","extInfo":{},"convertRate":"0.000015","createdAt":"1781092800000"},
                {"accountType":"funding_fiat","exchangeTxId":"f2","userId":"XXXXX","fromCoin":"ETH","fromCoinType":"crypto","fromAmount":"1","toCoin":"EUR","toCoinType":"fiat","toAmount":"2500","exchangeStatus":"failure","extInfo":{},"convertRate":"2500","createdAt":"1781179200000"}
            ]},"retExtInfo":{},"time":1720074457715}"#,
        )
        .unwrap();
        let record: Value = serde_json::from_str(
            r#"{"retCode":0,"retMsg":"OK","result":{"orderBody":[
                {"fromCoin":"BTC","fromAmount":"0.100000000000000000","toCoin":"ETH","toAmount":"1.385866230000000000","exchangeRate":"13.858662380000000000","createdTime":"1672197760","exchangeTxId":"145102533285208544812654440448"},
                {"fromCoin":"ETH","fromAmount":"1.000000000000000000","toCoin":"EUR","toAmount":"2400.000000000000000000","exchangeRate":"2400","createdTime":"1781265600","exchangeTxId":"r2"}
            ],"nextPageCursor":"173341:1672197760"},"retExtInfo":{},"time":1672990464021}"#,
        )
        .unwrap();
        let fiat_hist: Value = serde_json::from_str(
            r#"{"retCode":0,"retMsg":"success","result":[
                {"tradeNo":"TradeNo123456","status":"success","quoteTaxId":"QuoteTaxId123456","exchangeRate":"1.0","fromCoin":"GEL","fromCoinType":"fiat","toCoin":"USDT","toCoinType":"crypto","fromAmount":"100","toAmount":"100","createdAt":"1764560093588","subUserId":"123456"},
                {"tradeNo":"T2","status":"success","quoteTaxId":"Q2","exchangeRate":"0.000015","fromCoin":"EUR","fromCoinType":"fiat","toCoin":"BTC","toCoinType":"crypto","fromAmount":"200","toAmount":"0.003","createdAt":"1781092830000","subUserId":"123456"}
            ]}"#,
        )
        .unwrap();
        let mut recs = parse_convert_history(&history["result"]);
        assert_eq!(recs.len(), 4);
        recs.extend(parse_order_record(&record["result"]));
        recs.extend(parse_fiat_history(&fiat_hist["result"]));
        assert_eq!(recs.len(), 8);
        assert_eq!(recs[4].time_ms, 1672197760000);

        let mut acc = Acc::default();
        convert_flows(recs, 1767225600000, &mut acc);
        // A 2026 előtti dokumentációs példák a kezdődátum előttiek; a GEL nem MNB-deviza;
        // a sikertelen kimarad; a fiat Convert-beli EUR → BTC a query-convert-history ismétlése.
        let got: Vec<(String, f64)> = acc.flows.iter().map(|f| (f.date.to_string(), f.amount)).collect();
        assert_eq!(got, [("2026-06-10".to_string(), -200.0), ("2026-06-12".to_string(), 2400.0)]);
        assert_eq!(acc.ignored, 1);
        assert!(acc.flows[0].raw.contains("EUR 200 -> BTC 0.003 (funding_fiat)"), "{}", acc.flows[0].raw);

        let mut acc = Acc::default();
        convert_flows(parse_convert_history(&history["result"]), 0, &mut acc);
        // A két dokumentációs példa kripto–kripto, a harmadik sikertelen: mind kimarad.
        assert_eq!((acc.flows.len(), acc.ignored), (1, 3));
    }

    #[test]
    fn key_checks() {
        let doc: Value = serde_json::from_str(r#"{"id":"2208369","note":"testnet","apiKey":"XXXXXXXX","readOnly":1,"secret":"","uta":1}"#).unwrap();
        assert!(check_key(&doc).is_empty());
        assert_eq!(check_key(&serde_json::json!({"readOnly": 0, "uta": 0})).len(), 2);
    }

    #[test]
    fn error_messages() {
        assert!(matches!(api_error(10003, "API key is invalid."), Fail::BadKey));
        assert!(matches!(api_error(10004, "error sign!"), Fail::Fatal(m) if m.contains("titkos")));
        assert!(matches!(api_error(10005, "Permission denied"), Fail::Soft(_)));
    }

    /// A feldolgozott ügyletek a közös napi összevonással is helyes tételeket adnak.
    #[test]
    fn flows_aggregate() {
        let list: Value = serde_json::from_str(
            r#"[{"symbol":"BTCEUR","side":"Buy","execId":"e1","execPrice":"60000","execQty":"0.01","execValue":"600","execFee":"0.00001","feeCurrency":"BTC","execType":"Trade","execTime":"1775030400000"}]"#,
        )
        .unwrap();
        let mut acc = Acc::default();
        parse_executions(list.as_array().unwrap(), &pairs(), 0, &mut HashSet::new(), &mut acc);
        let p = acc.finish();
        assert_eq!((p.source, p.format), ("bybit", "Bybit – API"));
        let rows = super::super::aggregate(p.source, &p.flows);
        assert_eq!((rows[0].key.as_str(), rows[0].amount, rows[0].fee), ("bybit:2026-04-01:EUR:buy", 600.6, 0.6));
    }
}
