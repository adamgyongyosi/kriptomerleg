//! Binance fiat ↔ kripto ügyletek lekérése API-n.
//!
//! **Csak olvasó (read-only) API kulcs kell**: a Binance „API Management” oldalán elég az
//! „Enable Reading” jogosultság. Kereskedést, kifizetést, átvezetést engedélyező kulcsot ne adj
//! meg (ha mégis ilyet kapunk, figyelmeztetünk). A kulcsot és a titkot nem tároljuk: csak erre az
//! egy lekérésre használjuk.
//!
//! Lekért adatok (hivatalos dokumentáció: <https://developers.binance.com>):
//! - `GET /api/v3/exchangeInfo` (nyilvános) – a párok listája; csak azok kellenek, ahol az egyik
//!   oldal fiat (pl. BTCEUR, EURUSDT);
//! - `GET /api/v3/myTrades` – spot ügyletek páronként. A `startTime`/`endTime` ablak legfeljebb
//!   24 óra lehet, ezért inkább `fromId=0`-tól lapozunk (1000-es oldalak), és idő szerint szűrünk;
//! - `GET /sapi/v1/convert/tradeFlow` – Convert ügyletek, legfeljebb 30 napos ablakokban; ha egy
//!   ablakban több tétel van egy oldalnál (`moreData`), az ablakot megfelezzük;
//! - `GET /sapi/v1/fiat/payments` – kártyás / banki kriptóvásárlás (0) és -eladás (1), 30 napos
//!   ablakokban, oldalanként 500 tétellel; csak a `Completed` tételek számítanak.
//!
//! Lekérési korlát: a spot végpontok IP-súlykerete percenként 6000 (a `myTrades` 20-at ér), a
//! Convert előzmény 3000-et ér a felhasználónkénti 180 000/perc keretből. Kérések között várunk,
//! a válaszfejléc súlyszámlálóját figyeljük, és 429-es válasznál a `Retry-After` szerint várunk.
//!
//! A hálózati rész ([`fetch`]) és a válaszok feldolgozása (`parse_*`) külön van, hogy a
//! feldolgozás a dokumentáció példaválaszaival tesztelhető legyen.

use super::{FiatFlow, Parsed, budapest_date, fiat};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const API: &str = "https://api.binance.com";
const RECV_WINDOW: u32 = 10_000;
const DAY_MS: i64 = 86_400_000;
/// A `myTrades` oldalanként legfeljebb 1000 ügyletet ad.
const TRADES_LIMIT: usize = 1000;
/// Biztonsági korlát páronként (100 000 ügylet).
const MAX_TRADE_PAGES: usize = 100;
/// A Convert előzmény ablaka legfeljebb 30 nap lehet.
const CONVERT_WINDOW_MS: i64 = 30 * DAY_MS;
const CONVERT_LIMIT: usize = 1000;
/// Ennél rövidebb Convert-ablakot már nem felezünk tovább.
const MIN_CONVERT_WINDOW_MS: i64 = 3_600_000;
/// A fiat/payments alapértelmezett ablaka 30 nap; nagyobbat nem kérünk.
const PAYMENT_WINDOW_MS: i64 = 30 * DAY_MS;
const PAYMENT_ROWS: usize = 500;
const MAX_PAYMENT_PAGES: usize = 20;
/// Az összes kérés biztonsági korlátja.
const MAX_REQUESTS: usize = 5000;
/// A percenkénti IP-súlykeret 6000; e fölött megvárjuk a következő percet.
const WEIGHT_SOFT_LIMIT: u32 = 4800;

/// Hiba: a végzetes leállítja a lekérést (pl. rossz kulcs), az enyhe csak figyelmeztetés.
#[derive(Debug)]
enum Fail {
    Fatal(String),
    Soft(String),
}

impl From<Fail> for String {
    fn from(f: Fail) -> String {
        match f {
            Fail::Fatal(m) | Fail::Soft(m) => m,
        }
    }
}

/// Bájtok kisbetűs hexa alakja.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// signature = hex(HMAC-SHA256(lekérdezési szöveg, titok)).
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
    key: &'a str,
    secret: &'a str,
    /// A Binance szerveridő és a helyi óra különbsége (ms).
    offset_ms: i64,
    requests: usize,
}

impl Client<'_> {
    fn now_ms(&self) -> i64 {
        local_ms() + self.offset_ms
    }

    fn public(&mut self, path: &str, query: &str) -> Result<Value, Fail> {
        let url = if query.is_empty() { format!("{API}{path}") } else { format!("{API}{path}?{query}") };
        self.send(&url, false)
    }

    fn signed(&mut self, path: &str, query: &str) -> Result<Value, Fail> {
        let sep = if query.is_empty() { "" } else { "&" };
        let q = format!("{query}{sep}recvWindow={RECV_WINDOW}&timestamp={}", self.now_ms());
        let url = format!("{API}{path}?{q}&signature={}", sign(&q, self.secret));
        self.send(&url, true)
    }

    fn send(&mut self, url: &str, signed: bool) -> Result<Value, Fail> {
        for attempt in 0..4 {
            self.requests += 1;
            if self.requests > MAX_REQUESTS {
                return Err(Fail::Fatal("Túl sok Binance lekérés kellene; adj meg későbbi kezdődátumot.".into()));
            }
            let mut req = ureq::get(url).timeout(Duration::from_secs(30));
            if signed {
                req = req.set("X-MBX-APIKEY", self.key.trim());
            }
            match req.call() {
                Ok(resp) => {
                    let used = resp.header("X-MBX-USED-WEIGHT-1M").and_then(|w| w.trim().parse::<u32>().ok());
                    let json: Value = serde_json::from_reader(resp.into_reader())
                        .map_err(|e| Fail::Fatal(format!("Hibás Binance válasz: {e}")))?;
                    if used.is_some_and(|w| w >= WEIGHT_SOFT_LIMIT) {
                        // Közel a percenkénti súlykerethez: kivárjuk a következő percet.
                        pause((60_000 - local_ms().rem_euclid(60_000) + 500) as u64);
                    }
                    return Ok(json);
                }
                Err(ureq::Error::Status(429, resp)) if attempt < 3 => {
                    let wait = resp.header("Retry-After").and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(30);
                    std::thread::sleep(Duration::from_secs(wait.clamp(1, 120)));
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let body = resp.into_string().unwrap_or_default();
                    return Err(api_error(code, &body));
                }
                // Az URL-t (aláírással) nem írjuk ki, csak a hiba fajtáját.
                Err(e) => return Err(Fail::Fatal(format!("Nem sikerült elérni a Binance-t: {}", e.kind()))),
            }
        }
        Err(Fail::Fatal("A Binance túl sok kérést jelez (429). Próbáld újra néhány perc múlva.".into()))
    }
}

/// A Binance hibaválasza (`{"code":-2015,"msg":"..."}`) érthető magyar üzenetként.
fn api_error(status: u16, body: &str) -> Fail {
    let json: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let code = json["code"].as_i64().unwrap_or(0);
    let msg = json["msg"].as_str().unwrap_or("").trim();
    match code {
        -2014 | -2015 => Fail::Fatal(
            "A Binance elutasította a kulcsot: hibás API kulcs, IP-korlátozás, vagy nincs „Enable Reading” jogosultsága.".into(),
        ),
        -1022 => Fail::Fatal("A Binance elutasította az aláírást: hibás a titkos kulcs (Secret Key).".into()),
        -1021 => Fail::Fatal(
            "A számítógép órája eltér a Binance szerveridejétől. Állítsd az órát automatikus időszinkronra, majd próbáld újra.".into(),
        ),
        _ if status == 418 => {
            Fail::Fatal("A Binance ideiglenesen letiltotta ezt az IP-címet a túl sok kérés miatt. Próbáld újra később.".into())
        }
        _ if status == 451 => Fail::Fatal("A Binance erről a helyről nem szolgál ki API kéréseket (HTTP 451).".into()),
        0 => Fail::Soft(format!("Binance hiba (HTTP {status})")),
        _ => Fail::Soft(format!("Binance hiba ({code}): {msg}")),
    }
}

/* ───────────── Lekérés ───────────── */

/// Az összes fiat ↔ kripto ügylet `start_ms` (Unix ms) óta.
pub fn fetch(key: &str, secret: &str, start_ms: i64) -> Result<Parsed, String> {
    if key.trim().is_empty() || secret.trim().is_empty() {
        return Err("Add meg a Binance API kulcsot és a titkos kulcsot.".into());
    }
    let mut c = Client { key, secret, offset_ms: 0, requests: 0 };
    run(&mut c, start_ms).map_err(String::from)
}

fn run(c: &mut Client, start_ms: i64) -> Result<Parsed, Fail> {
    let mut acc = Acc::default();

    // Szerveridő: ha a gép órája elcsúszott, ahhoz igazítjuk az aláírt kérések időbélyegét.
    let time = c.public("/api/v3/time", "")?;
    if let Some(server) = time["serverTime"].as_i64() {
        c.offset_ms = server - local_ms();
    }

    // A kulcs ellenőrzése: itt derül ki, ha rossz vagy nem olvasó kulcs.
    match c.signed("/sapi/v1/account/apiRestrictions", "") {
        Ok(r) => acc.warnings.extend(check_restrictions(&r).map_err(Fail::Fatal)?),
        Err(Fail::Soft(m)) => acc.failed.push(format!("kulcs-jogosultságok: {m}")),
        Err(f) => return Err(f),
    }
    let now = c.now_ms();

    // 1. Spot ügyletek a fiat párokon.
    match c.public("/api/v3/exchangeInfo", "showPermissionSets=false") {
        Ok(info) => {
            let pairs = fiat_pairs(&info);
            for (i, p) in pairs.iter().enumerate() {
                if i > 0 {
                    pause(250);
                }
                fetch_trades(c, p, start_ms, &mut acc)?;
            }
        }
        Err(Fail::Soft(m)) => acc.failed.push(format!("párok listája (spot ügyletek): {m}")),
        Err(f) => return Err(f),
    }

    // 2. Convert.
    let mut seen = HashSet::new();
    let mut todo: Vec<(i64, i64)> = windows(start_ms, now, CONVERT_WINDOW_MS).into_iter().rev().collect();
    let mut first = true;
    while let Some((s, e)) = todo.pop() {
        if !first {
            // 3000-es súly a felhasználónkénti 180 000/perc keretből: legfeljebb ~55 kérés percenként.
            pause(1100);
        }
        first = false;
        match c.signed("/sapi/v1/convert/tradeFlow", &format!("startTime={s}&endTime={e}&limit={CONVERT_LIMIT}")) {
            Ok(v) => {
                if v["moreData"].as_bool() == Some(true) {
                    if e - s > MIN_CONVERT_WINDOW_MS {
                        let mid = s + (e - s) / 2;
                        todo.push((mid + 1, e));
                        todo.push((s, mid));
                        continue;
                    }
                    acc.truncated.push(format!("Convert {}", iso(s)));
                }
                parse_convert(&v, start_ms, &mut seen, &mut acc);
            }
            Err(Fail::Soft(m)) => {
                acc.failed.push(format!("Convert: {m}"));
                break;
            }
            Err(f) => return Err(f),
        }
    }

    // 3. Kártyás / banki vásárlás és eladás.
    let mut seen = HashSet::new();
    'types: for (tx_type, buy) in [(0, true), (1, false)] {
        for (s, e) in windows(start_ms, now, PAYMENT_WINDOW_MS) {
            for page in 1..=MAX_PAYMENT_PAGES {
                pause(200);
                let q = format!("transactionType={tx_type}&beginTime={s}&endTime={e}&page={page}&rows={PAYMENT_ROWS}");
                match c.signed("/sapi/v1/fiat/payments", &q) {
                    Ok(v) => {
                        let n = v["data"].as_array().map_or(0, Vec::len);
                        parse_payments(&v, buy, start_ms, &mut seen, &mut acc);
                        if n < PAYMENT_ROWS {
                            break;
                        }
                        if page == MAX_PAYMENT_PAGES {
                            acc.truncated.push(format!("kártyás/banki ügyletek {}", iso(s)));
                        }
                    }
                    Err(Fail::Soft(m)) => {
                        acc.failed.push(format!("kártyás/banki ügyletek: {m}"));
                        break 'types;
                    }
                    Err(f) => return Err(f),
                }
            }
        }
    }

    Ok(acc.finish())
}

/// Egy pár összes ügylete, `fromId=0`-tól lapozva (a `myTrades` 24 órás időablaka helyett).
fn fetch_trades(c: &mut Client, p: &Pair, start_ms: i64, acc: &mut Acc) -> Result<(), Fail> {
    let mut from_id: u64 = 0;
    for page in 0..MAX_TRADE_PAGES {
        if page > 0 {
            pause(250);
        }
        let q = format!("symbol={}&fromId={from_id}&limit={TRADES_LIMIT}", p.symbol);
        let trades = match c.signed("/api/v3/myTrades", &q) {
            Ok(v) => v,
            Err(Fail::Soft(m)) => {
                acc.failed.push(format!("{}: {m}", p.symbol));
                return Ok(());
            }
            Err(f) => return Err(f),
        };
        let list = trades.as_array().map(Vec::as_slice).unwrap_or(&[]);
        parse_trades(p, list, start_ms, acc);
        if list.len() < TRADES_LIMIT {
            return Ok(());
        }
        from_id = list.iter().filter_map(|t| t["id"].as_u64()).max().unwrap_or(from_id) + 1;
    }
    acc.truncated.push(p.symbol.clone());
    Ok(())
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
    /// Díj harmadik eszközben (pl. BNB), aminek nincs árfolyama az ügyletben.
    fee_unknown: usize,
    /// Kártyás/banki eladások (a mezők jelentése a dokumentációban nem egyértelmű).
    card_sells: usize,
    /// „Cash Balance” (Binance-egyenlegből fizetett) vásárlások.
    cash_balance: usize,
    /// Sikertelen (nem végzetes) lekérések.
    failed: Vec<String>,
    /// Biztonsági korlát miatt csonka lekérések.
    truncated: Vec<String>,
}

impl Acc {
    fn finish(mut self) -> Parsed {
        if self.fee_unknown > 0 {
            self.warnings.push(format!(
                "{} ügyletnél a díjat harmadik eszközben (pl. BNB) vonták le; ennek fiat értéke az ügyletből nem számolható, ezért 0-nak vettem. Ha számít, rögzítsd kézzel.",
                self.fee_unknown
            ));
        }
        if self.card_sells > 0 {
            self.warnings.push(format!(
                "{} kártyás/banki eladást találtam. A Binance dokumentáció ezek mezőit nem részletezi; a fiat összeget az árfolyam alapján azonosítottam – érdemes a Binance felületén ellenőrizni.",
                self.card_sells
            ));
        }
        if self.cash_balance > 0 {
            self.warnings.push(format!(
                "{} vásárlást a Binance-en tartott fiat egyenlegből („Cash Balance”) fizettél. Ha ugyanez Convert ügyletként is megjelenik a listában, az egyiket töröld.",
                self.cash_balance
            ));
        }
        if !self.failed.is_empty() {
            let shown: Vec<&str> = self.failed.iter().take(5).map(String::as_str).collect();
            let more = if self.failed.len() > 5 { format!(" (és még {})", self.failed.len() - 5) } else { String::new() };
            self.warnings.push(format!(
                "Néhány lekérés nem sikerült, ezek ügyletei hiányozhatnak: {}{more}. Ezeket fájlexportból importáld.",
                shown.join("; ")
            ));
        }
        if !self.truncated.is_empty() {
            self.warnings.push(format!(
                "Túl sok tétel miatt a lekérés csonka lehet: {}. A hiányzó részt fájlexportból importáld.",
                self.truncated.join(", ")
            ));
        }
        Parsed { source: "binance", format: "Binance – API", flows: self.flows, ignored: self.ignored, warnings: self.warnings }
    }

    fn push(&mut self, ms: i64, currency: String, amount: f64, fee: f64, raw: String) {
        match date_of(ms) {
            Some(date) => self.flows.push(FiatFlow { date, currency, amount, fee, raw, ..Default::default() }),
            None => self.ignored += 1,
        }
    }
}

/// Szám a JSON-ból (a Binance szövegként adja: `"0.00500000"`).
fn number(v: &Value) -> Option<f64> {
    match v {
        Value::String(s) => s.trim().parse().ok(),
        Value::Number(n) => n.as_f64(),
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

/// A kulcs jogosultságai (`/sapi/v1/account/apiRestrictions`): olvasás kell, írás nem ajánlott.
fn check_restrictions(r: &Value) -> Result<Vec<String>, String> {
    if r["enableReading"].as_bool() == Some(false) {
        return Err("A kulcsnak nincs „Enable Reading” jogosultsága.".into());
    }
    const WRITE: &[&str] = &[
        "enableSpotAndMarginTrading",
        "enableWithdrawals",
        "enableInternalTransfer",
        "enableMargin",
        "enableFutures",
        "permitsUniversalTransfer",
        "enableVanillaOptions",
        "enablePortfolioMarginTrading",
        "enableFixApiTrade",
    ];
    let writes: Vec<&str> = WRITE.iter().copied().filter(|k| r[*k].as_bool() == Some(true)).collect();
    Ok(if writes.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "A megadott kulcs nem csak olvasó ({}). Biztonsági okból hozz létre csak „Enable Reading” jogú kulcsot.",
            writes.join(", ")
        )]
    })
}

/// Egy pár, amelynek pontosan az egyik oldala fiat.
#[derive(Debug, Clone, PartialEq)]
struct Pair {
    symbol: String,
    base: String,
    quote: String,
}

/// A fiat párok az `exchangeInfo` válaszból (minden státusszal, hogy a kivezetett párok is meglegyenek).
fn fiat_pairs(info: &Value) -> Vec<Pair> {
    info["symbols"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|s| {
            let base = s["baseAsset"].as_str()?.to_uppercase();
            let quote = s["quoteAsset"].as_str()?.to_uppercase();
            let symbol = s["symbol"].as_str()?.to_string();
            (fiat(&base).is_some() != fiat(&quote).is_some()).then_some(Pair { symbol, base, quote })
        })
        .collect()
}

/// `myTrades` válasz → FiatFlow-k. A `start_ms` előtti ügyleteket kihagyjuk.
fn parse_trades(p: &Pair, trades: &[Value], start_ms: i64, acc: &mut Acc) {
    let (currency, fiat_is_quote) = match (fiat(&p.base), fiat(&p.quote)) {
        (None, Some(q)) => (q, true),
        (Some(b), None) => (b, false),
        _ => {
            acc.ignored += trades.len();
            return;
        }
    };
    for t in trades {
        let Some(time) = t["time"].as_i64() else {
            acc.ignored += 1;
            continue;
        };
        if time < start_ms {
            continue;
        }
        let (Some(qty), Some(quote_qty), Some(price)) = (number(&t["qty"]), number(&t["quoteQty"]), number(&t["price"])) else {
            acc.ignored += 1;
            continue;
        };
        let buyer = t["isBuyer"].as_bool().unwrap_or(false);
        // A fiat oldal bruttó összege, és hogy fiat ment-e ki.
        // BTCEUR: vételnél EUR megy ki; EURUSDT: vételnél EUR jön be (kriptót adtunk el érte).
        let (gross, fiat_out) = if fiat_is_quote { (quote_qty.abs(), buyer) } else { (qty.abs(), !buyer) };
        let commission = number(&t["commission"]).unwrap_or(0.0).abs();
        let asset = t["commissionAsset"].as_str().unwrap_or("").to_uppercase();
        let fee = if commission == 0.0 {
            0.0
        } else if fiat(&asset).as_deref() == Some(currency.as_str()) {
            commission
        } else if asset == p.base && fiat_is_quote {
            // A díj a megvett kriptóban: az ügylet árfolyamán átváltva.
            commission * price
        } else if asset == p.quote && !fiat_is_quote && price > 0.0 {
            commission / price
        } else {
            acc.fee_unknown += 1;
            0.0
        };
        let amount = if fiat_out { -(gross + fee) } else { gross - fee };
        let raw = format!(
            "myTrades {} id={} {} {} qty={} quoteQty={} price={} commission={} {}",
            p.symbol,
            text(&t["id"]),
            iso(time),
            if buyer { "buy" } else { "sell" },
            text(&t["qty"]),
            text(&t["quoteQty"]),
            text(&t["price"]),
            text(&t["commission"]),
            asset
        );
        acc.push(time, currency.clone(), amount, fee, raw);
    }
}

/// `convert/tradeFlow` válasz → FiatFlow-k. A Convert díja az árfolyamban van, külön díj nincs.
fn parse_convert(v: &Value, start_ms: i64, seen: &mut HashSet<String>, acc: &mut Acc) {
    for r in v["list"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
        let id = text(&r["orderId"]);
        if !id.is_empty() && !seen.insert(id.clone()) {
            continue;
        }
        let Some(time) = r["createTime"].as_i64() else {
            acc.ignored += 1;
            continue;
        };
        if time < start_ms {
            continue;
        }
        if r["orderStatus"].as_str() != Some("SUCCESS") {
            acc.ignored += 1;
            continue;
        }
        let (from, to) = (r["fromAsset"].as_str().unwrap_or(""), r["toAsset"].as_str().unwrap_or(""));
        let raw = format!(
            "convert/tradeFlow orderId={id} {} {} {} -> {} {}",
            iso(time),
            from,
            text(&r["fromAmount"]),
            to,
            text(&r["toAmount"])
        );
        match (fiat(from), fiat(to), number(&r["fromAmount"]), number(&r["toAmount"])) {
            (Some(c), None, Some(a), _) => acc.push(time, c, -a.abs(), 0.0, raw),
            (None, Some(c), _, Some(a)) => acc.push(time, c, a.abs(), 0.0, raw),
            _ => acc.ignored += 1,
        }
    }
}

/// `fiat/payments` válasz → FiatFlow-k (`buy`: transactionType 0, különben 1).
///
/// Vételnél a dokumentáció példája szerint `sourceAmount` a kifizetett fiat (díjjal együtt),
/// `obtainAmount` a kapott kripto. Eladásnál a dokumentáció nem részletezi a mezőket; a Binance
/// fórumon adott válasz szerint fordítva van (`sourceAmount` kripto, `obtainAmount` fiat). Ezért az
/// árfolyamból (`price`, fiat/kripto) ellenőrizzük, melyik a fiat oldal, és hogy a díj benne van-e.
fn parse_payments(v: &Value, buy: bool, start_ms: i64, seen: &mut HashSet<String>, acc: &mut Acc) {
    for p in v["data"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
        let id = text(&p["orderNo"]);
        if !id.is_empty() && !seen.insert(id.clone()) {
            continue;
        }
        let Some(time) = p["createTime"].as_i64() else {
            acc.ignored += 1;
            continue;
        };
        if time < start_ms {
            continue;
        }
        let Some(currency) = p["fiatCurrency"].as_str().and_then(fiat) else {
            acc.ignored += 1;
            continue;
        };
        let (Some(source), Some(obtain)) = (number(&p["sourceAmount"]), number(&p["obtainAmount"])) else {
            acc.ignored += 1;
            continue;
        };
        if p["status"].as_str() != Some("Completed") {
            acc.ignored += 1;
            continue;
        }
        if p["paymentMethod"].as_str().is_some_and(|m| m.eq_ignore_ascii_case("Cash Balance")) {
            acc.cash_balance += 1;
        }
        let fee = number(&p["totalFee"]).unwrap_or(0.0).abs();
        let price = number(&p["price"]).filter(|x| *x > 0.0);
        let (fiat_amt, crypto_amt) = payment_sides(buy, source.abs(), obtain.abs(), price);
        let gross = price.map(|pr| crypto_amt * pr);
        let amount = if buy {
            // Benne van-e a díj a fizetett összegben? (A dokumentáció példájában igen.)
            match gross {
                Some(g) if (g - fiat_amt).abs() < (g + fee - fiat_amt).abs() => -(fiat_amt + fee),
                _ => -fiat_amt,
            }
        } else {
            acc.card_sells += 1;
            // Nettó (díj utáni) összeg-e a kapott fiat?
            match gross {
                Some(g) if (g - fiat_amt).abs() < (g - fee - fiat_amt).abs() => fiat_amt - fee,
                _ => fiat_amt,
            }
        };
        let raw = format!(
            "fiat/payments orderNo={id} {} {} {} sourceAmount={} obtainAmount={} {} {} price={} totalFee={} {}",
            iso(time),
            if buy { "buy" } else { "sell" },
            text(&p["paymentMethod"]),
            text(&p["sourceAmount"]),
            text(&p["obtainAmount"]),
            text(&p["cryptoCurrency"]),
            currency,
            text(&p["price"]),
            text(&p["totalFee"]),
            currency
        );
        acc.push(time, currency, amount, fee, raw);
    }
}

/// (fiat összeg, kripto mennyiség). Alapesetben vételnél a `source`, eladásnál az `obtain` a fiat;
/// ha az árfolyam egyértelműen a fordítottját mutatja, megcseréljük.
fn payment_sides(buy: bool, source: f64, obtain: f64, price: Option<f64>) -> (f64, f64) {
    let (fiat_amt, crypto_amt) = if buy { (source, obtain) } else { (obtain, source) };
    let Some(pr) = price else { return (fiat_amt, crypto_amt) };
    let err = |f: f64, c: f64| if f > 0.0 { (c * pr - f).abs() / f } else { f64::INFINITY };
    if err(fiat_amt, crypto_amt) > 0.1 && err(crypto_amt, fiat_amt) < 0.1 { (crypto_amt, fiat_amt) } else { (fiat_amt, crypto_amt) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Binance dokumentáció HMAC aláírás-példája (SIGNED Endpoint Examples, HMAC Keys).
    #[test]
    fn signature_matches_binance_docs() {
        let secret = "NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j";
        let payload = "symbol=LTCBTC&side=BUY&type=LIMIT&timeInForce=GTC&quantity=1&price=0.1&recvWindow=5000&timestamp=1499827319559";
        assert_eq!(sign(payload, secret), "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71");
    }

    #[test]
    fn time_windows_do_not_overlap() {
        let w = windows(0, 2 * CONVERT_WINDOW_MS + 5, CONVERT_WINDOW_MS);
        assert_eq!(w.len(), 3);
        assert_eq!(w[0], (0, CONVERT_WINDOW_MS - 1));
        assert_eq!(w[1].0, CONVERT_WINDOW_MS);
        assert_eq!(w[2].1, 2 * CONVERT_WINDOW_MS + 5);
        assert!(w.iter().all(|(s, e)| e - s < CONVERT_WINDOW_MS));
    }

    /// A dokumentáció példája (ETHBTC) kimarad; a fiat párok (BTCEUR, EURUSDT) bekerülnek.
    #[test]
    fn exchange_info_fiat_pairs() {
        let info: Value = serde_json::from_str(
            r#"{"timezone":"UTC","serverTime":1565246363776,"symbols":[
                {"symbol":"ETHBTC","status":"TRADING","baseAsset":"ETH","baseAssetPrecision":8,"quoteAsset":"BTC","quotePrecision":8},
                {"symbol":"BTCEUR","status":"TRADING","baseAsset":"BTC","quoteAsset":"EUR"},
                {"symbol":"EURUSDT","status":"TRADING","baseAsset":"EUR","quoteAsset":"USDT"},
                {"symbol":"BTCPLN","status":"BREAK","baseAsset":"BTC","quoteAsset":"PLN"}
            ]}"#,
        )
        .unwrap();
        let pairs = fiat_pairs(&info);
        let names: Vec<&str> = pairs.iter().map(|p| p.symbol.as_str()).collect();
        assert_eq!(names, ["BTCEUR", "EURUSDT", "BTCPLN"]);
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn pair(symbol: &str, base: &str, quote: &str) -> Pair {
        Pair { symbol: symbol.into(), base: base.into(), quote: quote.into() }
    }

    /// A `myTrades` dokumentációs példájának mezőivel.
    #[test]
    fn spot_trades() {
        // A dokumentáció példája (BNBBTC) kripto–kripto: kimarad.
        let doc: Value = serde_json::from_str(
            r#"[{"symbol":"BNBBTC","id":28457,"orderId":100234,"orderListId":-1,"price":"4.00000100","qty":"12.00000000","quoteQty":"48.000012","commission":"10.10000000","commissionAsset":"BNB","time":1499865549590,"isBuyer":true,"isMaker":false,"isBestMatch":true}]"#,
        )
        .unwrap();
        let mut acc = Acc::default();
        parse_trades(&pair("BNBBTC", "BNB", "BTC"), doc.as_array().unwrap(), 0, &mut acc);
        assert_eq!((acc.flows.len(), acc.ignored), (0, 1));

        // 2026-03-04 vétel (díj BTC-ben), 2026-06-01 eladás (díj EUR-ban), BNB-díjas vétel,
        // és egy kezdődátum előtti ügylet, ami kimarad.
        let trades: Value = serde_json::from_str(
            r#"[
            {"symbol":"BTCEUR","id":1,"orderId":10,"orderListId":-1,"price":"60000.00","qty":"0.01000000","quoteQty":"600.00","commission":"0.00001000","commissionAsset":"BTC","time":1709546700000,"isBuyer":true,"isMaker":false,"isBestMatch":true},
            {"symbol":"BTCEUR","id":2,"orderId":11,"orderListId":-1,"price":"80000.00","qty":"0.00500000","quoteQty":"400.00","commission":"0.40000000","commissionAsset":"EUR","time":1772618700000,"isBuyer":true,"isMaker":false,"isBestMatch":true},
            {"symbol":"BTCEUR","id":3,"orderId":12,"orderListId":-1,"price":"70000.00","qty":"0.00300000","quoteQty":"210.00","commission":"0.21000000","commissionAsset":"EUR","time":1780315200000,"isBuyer":false,"isMaker":true,"isBestMatch":true},
            {"symbol":"BTCEUR","id":4,"orderId":13,"orderListId":-1,"price":"70000.00","qty":"0.00100000","quoteQty":"70.00","commission":"0.00010000","commissionAsset":"BNB","time":1780315300000,"isBuyer":true,"isMaker":true,"isBestMatch":true}
        ]"#,
        )
        .unwrap();
        let start = 1767225600000; // 2026-01-01
        let mut acc = Acc::default();
        parse_trades(&pair("BTCEUR", "BTC", "EUR"), trades.as_array().unwrap(), start, &mut acc);
        assert_eq!(acc.flows.len(), 3);
        let buy = &acc.flows[0];
        assert_eq!((buy.date.to_string(), buy.currency.as_str()), ("2026-03-04".into(), "EUR"));
        assert!(close(buy.amount, -400.4) && close(buy.fee, 0.4));
        assert!(buy.raw.starts_with("myTrades BTCEUR id=2 2026-03-04T10:05:00Z buy qty=0.00500000"), "{}", buy.raw);
        let sell = &acc.flows[1];
        assert!(close(sell.amount, 209.79) && close(sell.fee, 0.21));
        assert_eq!((acc.flows[2].amount, acc.flows[2].fee, acc.fee_unknown), (-70.0, 0.0, 1));

        // Kriptóban levont díj az ügylet árfolyamán: 0.00001 BTC × 60000 = 0.6 EUR.
        let mut acc = Acc::default();
        parse_trades(&pair("BTCEUR", "BTC", "EUR"), &trades.as_array().unwrap()[..1], 0, &mut acc);
        assert!(close(acc.flows[0].amount, -600.6) && close(acc.flows[0].fee, 0.6));
    }

    /// EURUSDT: az EUR vétele USDT-ért fiat bevétel, az EUR eladása fiat kiadás.
    #[test]
    fn fiat_as_base_asset() {
        let trades: Value = serde_json::from_str(
            r#"[
            {"symbol":"EURUSDT","id":7,"price":"1.10","qty":"100.00","quoteQty":"110.00","commission":"0.10","commissionAsset":"EUR","time":1772618700000,"isBuyer":true},
            {"symbol":"EURUSDT","id":8,"price":"1.10","qty":"50.00","quoteQty":"55.00","commission":"0.055","commissionAsset":"USDT","time":1772618800000,"isBuyer":false}
        ]"#,
        )
        .unwrap();
        let mut acc = Acc::default();
        parse_trades(&pair("EURUSDT", "EUR", "USDT"), trades.as_array().unwrap(), 0, &mut acc);
        assert!(close(acc.flows[0].amount, 99.9) && close(acc.flows[0].fee, 0.1));
        assert!(close(acc.flows[1].amount, -50.05) && close(acc.flows[1].fee, 0.05));
    }

    /// A `convert/tradeFlow` dokumentációs példája (USDT → BNB) kimarad; a fiat oldalúak bekerülnek.
    #[test]
    fn convert_trade_flow() {
        let v: Value = serde_json::from_str(
            r#"{"list":[
                {"quoteId":"f3b91c525b2644c7bc1e1cd31b6e1aa6","orderId":940708407462087195,"orderStatus":"SUCCESS","fromAsset":"USDT","fromAmount":"20","toAsset":"BNB","toAmount":"0.06154036","ratio":"0.00307702","inverseRatio":"324.99","createTime":1624248872184},
                {"quoteId":"a1","orderId":1,"orderStatus":"SUCCESS","fromAsset":"EUR","fromAmount":"200","toAsset":"BTC","toAmount":"0.003","ratio":"0.000015","inverseRatio":"66666.67","createTime":1781092800000},
                {"quoteId":"a2","orderId":2,"orderStatus":"SUCCESS","fromAsset":"ETH","fromAmount":"1","toAsset":"EUR","toAmount":"2500","ratio":"2500","inverseRatio":"0.0004","createTime":1781179200000},
                {"quoteId":"a3","orderId":3,"orderStatus":"FAIL","fromAsset":"EUR","fromAmount":"50","toAsset":"BTC","toAmount":"0.0007","ratio":"0.000014","inverseRatio":"71428","createTime":1781179300000},
                {"quoteId":"a1","orderId":1,"orderStatus":"SUCCESS","fromAsset":"EUR","fromAmount":"200","toAsset":"BTC","toAmount":"0.003","ratio":"0.000015","inverseRatio":"66666.67","createTime":1781092800000}
            ],"startTime":1623824139000,"endTime":1626416139000,"limit":100,"moreData":false}"#,
        )
        .unwrap();
        let mut acc = Acc::default();
        parse_convert(&v, 0, &mut HashSet::new(), &mut acc);
        assert_eq!(acc.ignored, 2);
        let amounts: Vec<f64> = acc.flows.iter().map(|f| f.amount).collect();
        assert_eq!(amounts, [-200.0, 2500.0]);
        assert_eq!(acc.flows[0].date.to_string(), "2026-06-10");
        assert!(acc.flows[1].raw.contains("ETH 1 -> EUR 2500"));
    }

    /// A `fiat/payments` dokumentációs példája sikertelen (Failed) – kimarad; a sikeresek bekerülnek.
    #[test]
    fn card_payments() {
        let doc: Value = serde_json::from_str(
            r#"{"code":"000000","message":"success","data":[
                {"orderNo":"353fca443f06466db0c4dc89f94f027a","sourceAmount":"20.0","fiatCurrency":"EUR","obtainAmount":"4.462","cryptoCurrency":"LUNA","totalFee":"0.2","price":"4.437472","status":"Failed","paymentMethod":"Credit Card","createTime":1624529919000,"updateTime":1624529919000}
            ],"total":1,"success":true}"#,
        )
        .unwrap();
        let mut acc = Acc::default();
        parse_payments(&doc, true, 0, &mut HashSet::new(), &mut acc);
        assert_eq!((acc.flows.len(), acc.ignored), (0, 1));

        // Ugyanez sikeresen: 20 EUR fizetve, ebből 0.2 díj (4.462 × 4.437472 + 0.2 ≈ 20).
        let ok = serde_json::to_string(&doc).unwrap().replace("\"Failed\"", "\"Completed\"");
        let mut acc = Acc::default();
        parse_payments(&serde_json::from_str(&ok).unwrap(), true, 0, &mut HashSet::new(), &mut acc);
        assert_eq!((acc.flows[0].amount, acc.flows[0].fee, acc.flows[0].currency.as_str()), (-20.0, 0.2, "EUR"));
        assert!(acc.flows[0].raw.contains("buy Credit Card"));

        // Eladás a fórumon leírt mezőkiosztással: 4.462 LUNA → 20 EUR, díj 0.2 EUR.
        let sell: Value = serde_json::from_str(
            r#"{"code":"000000","message":"success","data":[
                {"orderNo":"s1","sourceAmount":"4.462","fiatCurrency":"EUR","obtainAmount":"19.8","cryptoCurrency":"LUNA","totalFee":"0.2","price":"4.48229","status":"Completed","createTime":1624529919000,"updateTime":1624529919000}
            ],"total":1,"success":true}"#,
        )
        .unwrap();
        let mut acc = Acc::default();
        parse_payments(&sell, false, 0, &mut HashSet::new(), &mut acc);
        assert!(close(acc.flows[0].amount, 19.8) && close(acc.flows[0].fee, 0.2) && acc.card_sells == 1);
    }

    #[test]
    fn payment_side_detection() {
        // Ha az eladásnál mégis a source a fiat, az árfolyam alapján megcseréljük.
        assert_eq!(payment_sides(false, 20.0, 4.462, Some(4.48)), (20.0, 4.462));
        assert_eq!(payment_sides(false, 4.462, 20.0, Some(4.48)), (20.0, 4.462));
        assert_eq!(payment_sides(true, 20.0, 4.462, None), (20.0, 4.462));
    }

    #[test]
    fn key_permissions() {
        let read_only: Value = serde_json::from_str(
            r#"{"ipRestrict":false,"createTime":1623840271000,"enableReading":true,"enableWithdrawals":false,"enableInternalTransfer":false,"enableMargin":false,"enableFutures":false,"permitsUniversalTransfer":false,"enableVanillaOptions":false,"enableFixApiTrade":false,"enableFixReadOnly":true,"enableSpotAndMarginTrading":false,"enablePortfolioMarginTrading":false}"#,
        )
        .unwrap();
        assert!(check_restrictions(&read_only).unwrap().is_empty());
        // A dokumentáció példája belső átvezetést is enged → figyelmeztetés.
        let doc: Value = serde_json::from_str(
            r#"{"ipRestrict":false,"createTime":1623840271000,"enableReading":true,"enableWithdrawals":false,"enableInternalTransfer":true,"enableMargin":false,"enableFutures":false,"permitsUniversalTransfer":true,"enableVanillaOptions":false,"enableFixApiTrade":false,"enableFixReadOnly":true,"enableSpotAndMarginTrading":false,"enablePortfolioMarginTrading":true}"#,
        )
        .unwrap();
        assert_eq!(check_restrictions(&doc).unwrap().len(), 1);
        assert!(check_restrictions(&serde_json::json!({"enableReading": false})).is_err());
    }

    #[test]
    fn error_messages() {
        assert!(matches!(api_error(401, r#"{"code":-2015,"msg":"Invalid API-key, IP, or permissions for action."}"#), Fail::Fatal(m) if m.contains("kulcsot")));
        assert!(matches!(api_error(400, r#"{"code":-1022,"msg":"Signature for this request is not valid."}"#), Fail::Fatal(m) if m.contains("titkos")));
        assert!(matches!(api_error(400, r#"{"code":-1121,"msg":"Invalid symbol."}"#), Fail::Soft(_)));
    }

    /// A feldolgozott ügyletek a közös napi összevonással is helyes tételeket adnak.
    #[test]
    fn flows_aggregate() {
        let v: Value = serde_json::from_str(
            r#"[{"symbol":"BTCEUR","id":2,"price":"80000","qty":"0.005","quoteQty":"400","commission":"0.4","commissionAsset":"EUR","time":1772618700000,"isBuyer":true}]"#,
        )
        .unwrap();
        let mut acc = Acc::default();
        parse_trades(&pair("BTCEUR", "BTC", "EUR"), v.as_array().unwrap(), 0, &mut acc);
        let p = acc.finish();
        assert_eq!((p.source, p.format), ("binance", "Binance – API"));
        let rows = super::super::aggregate(p.source, &p.flows);
        assert_eq!((rows[0].key.as_str(), rows[0].amount, rows[0].fee), ("binance:2026-03-04:EUR:buy", 400.4, 0.4));
    }
}
