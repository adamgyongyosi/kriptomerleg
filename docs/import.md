# Tőzsdei import

[← vissza a README-hez](../README.md)

*Importálás* (bal alsó sarok) → export fájl kiválasztása → előnézet → *Importálás*.

| Tőzsde | Honnan töltsd le | Mit olvas be |
|---|---|---|
| Binance | Wallet → Transaction History → Export (Generate all statements) | A fiat egyenleg minden ügyletből eredő változása (Transaction Spend/Revenue/Fee, Convert, Buy/Sell…) |
| Revolut | Kripto → … → Statement (kivonat), CSV vagy Excel | `Buy` / `Sell` sorok értéke és díja (újabb formátum); `EXCHANGE` sorok fiat összege (régi formátum) |
| Kraken | History → Export → Ledgers | `trade` / `spend` / `receive` sorok fiat összege, díj levonva |
| Bybit | Orders → Spot Order → Trade History → Export; Convert → History → Export | Fiat párok (pl. BTCEUR) kereskedései díjjal; fiat ↔ kripto konvertálások |
| Crypto.com | App → Accounts → Transaction History → Export, „Crypto Wallet” (`crypto_transactions_record`) | `card_top_up` (Visa kártya feltöltése kriptóból) „Fizetés kriptóval” tételként a `Native Amount` összegével; fiat → kripto vétel (`viban_purchase`, `crypto_purchase`) és kripto → fiat eladás (`crypto_viban_exchange`). Cashback, visszatérítés, fiatból töltött kártya, kripto–kripto csere kimarad. |
| Bank (Revolut, Wise, magyar bankok) | a netbank számlakivonat-exportja (CSV/Excel) | A kriptotőzsdékre (Binance, Kraken/Payward, Bybit, Coinbase, Bitstamp, Bitpanda, OKX, KuCoin, Crypto.com, …) küldött utalás = befizetés, onnan érkező = kivétel. Közelítés: ha van tőzsdei export, az pontosabb. |

**API-s lekérés (Binance, Bybit, Kraken):** az Importálás ablakban a „Lekérés közvetlenül, API kulccsal” alatt választható a tőzsde. A kulcsnak csak olvasási joga legyen; a kulcsot és a titkot az app nem menti, csak az adott lekérésre használja. Az eredmény ugyanúgy előnézetbe kerül, és ugyanazzal a kulccsal (`binance:…`, `bybit:…`, `kraken:…`) mentődik, mint a fájlos import, így a kettő nem duplikál.

- **Kraken:** a főkönyvet (Ledgers) kéri le, és ugyanúgy dolgozza fel, mint a Ledgers exportot. Jog: „Query Ledger Entries”. Oldalanként 50 sor, a Kraken lekérési korlátja miatt oldalanként kb. 2,5 mp.
- **Binance** és **Bybit:** lásd lent, [API-s lekérés részletei](#api-s-lekérés-részletei).

A formátumot az oszlopnevek alapján ismeri fel. A menük neve a tőzsdei appok frissítésével változhat.

## API-s lekérés részletei

Mindkét modul (`binance_api.rs`, `bybit_api.rs`) a tőzsde szerveridejéhez igazítja az aláírás időbélyegét, lekéréskor ellenőrzi, hogy a kulcs csak olvasó-e (ha írási joga is van, figyelmeztet), és tartja a tőzsde lekérési korlátait (várakozás, 429-es válasznál újrapróbálás). A hibás kulcsról, aláírásról, órahibáról és IP-tiltásról magyar hibaüzenetet ad; ha csak egy végpont nem érhető el a fióknak, a többit lekéri, és figyelmeztet. Biztonsági korlát: legfeljebb 5000 kérés.

**Binance** (kulcs: API Management → rendszer által generált, csak „Enable Reading”):

| Végpont | Mit ad |
|---|---|
| `/api/v3/myTrades` | spot ügyletek minden olyan páron, ahol az egyik oldal fiat (a párlistát az `exchangeInfo` adja); a díj, ha kriptóban van, az ügylet árfolyamán számolva |
| `/sapi/v1/convert/tradeFlow` | Convert ügyletek fiat ↔ kripto (30 napos ablakokkal) |
| `/sapi/v1/fiat/payments` | bankkártyás/banki kriptóvásárlás és -eladás (csak a teljesítettek) |

Ismert korlátok: a P2P és a margin ügyletek nincsenek benne; a teljesen kivezetett párok ügyletei nem kérhetők le; a `fiat/payments` eladásainak mezőit a dokumentáció nem részletezi (az app az árfolyamból dönti el, melyik a fiat összeg, és figyelmeztet, hogy érdemes ellenőrizni). Sok fiat pár esetén a lekérés fél percnél tovább tarthat.

**Bybit** (kulcs: API → Create New Key → System-generated, „Read-Only”):

| Végpont | Mit ad |
|---|---|
| `/v5/execution/list?category=spot` | spot ügyletek fiat párokon (7 napos ablakokkal); a Bybit API legfeljebb kb. 2 évre visszamenőleg ad adatot – a régebbit fájlexportból importáld |
| `/v5/asset/exchange/query-convert-history`, `/v5/asset/exchange/order-record`, `/v5/fiat/query-trade-history` | Convert ügyletek fiat ↔ kripto (az átfedő források ismétlődését kiszűri) |

Ismert korlátok: az EGT-s (bybit.eu) fiókok API-ja a Bybit dokumentációja szerint csak brókeres, harmadik feles alkalmazásoknak érhető el, így magyar fióknál előfordulhat, hogy a lekérés nem működik – ilyenkor a fájlexport a megoldás. Klasszikus (nem Unified) fióknál a régi spot ügyletek hiányosak lehetnek.

Mindkét API-modult a hivatalos dokumentáció példaválaszaival teszteltük; valódi kulccsal végzett lekérés eredményét az előnézetben érdemes összevetni a tőzsde felületével.

**Példafájlok:** az [`examples/`](../examples/) mappában minden formátumhoz van egy kitalált adatokkal kitöltött, importálható minta (és a régi Excel nyilvántartás mintája is). A [`examples/README.md`](../examples/README.md) leírja, melyik fájl mit mutat be, és milyen szerkezetű lehet a saját exportod.

**Hogyan számol:**

- Csak a **fiat ↔ kripto** ügyletek számítanak: vétel fiatért = kiadás, eladás fiatra = bevétel. A tőzsdére utalt vagy onnan kivett pénz, és a kripto–kripto csere (pl. BTC → USDT) nem adóköteles, ezeket kihagyja.
- A **díjak** a kiadást növelik, illetve a bevételt csökkentik. Ha a díj kriptóban van (pl. Bybit vételnél), az ügylet árfolyamán számítja át.
- Az ügyleteket **naponta, devizánként és irányonként összevonja** egy tételbe; a forintra váltás az aznapi MNB árfolyammal történik (USD-t, GBP-t stb. is kezel).
- A mentés **egyetlen adatbázis-tranzakcióban** történik: több ezer soros fájlnál is egy lemezírás, és ha közben hiba lép fel, egyik tétel sem kerül be félig.
- A tőzsdék UTC-ben adják az időt, az app ezt magyar időre (CET/CEST) váltja a nap meghatározásához.
- **Újraimportálás** (pl. hosszabb időszakkal) nem duplikál: a már meglévő napi tételeket frissíti.
- Ha az adott évben **kézi vagy Excel-tétel** is van, figyelmeztet: ha azok ugyanazokat a vásárlásokat jelentik (pl. a tőzsdére utalt euró), töröld őket, különben a kiadás duplán számít.
- A *Tételek* oldalon forrás szerint szűrhetsz, és egy forrás adott évi tételeit egyben törölheted.
- Figyelmeztet, ha olyan sort talál, amit kézzel kell rögzítened (pl. Binance kártyás vásárlás fiat összeg nélkül, P2P).
- **Kriptokártyák:** adózási szempontból az a pillanat számít, amikor a kripto fiattá válik. A Crypto.com Visa kártyánál ez a kriptós kártyafeltöltés (`card_top_up`), a Binance kártyánál a fizetés („Binance Card Spending” kriptó sora). A fiat egyenlegből fizetett kártyás vásárlás nem kriptós ügylet. A Bybit és a Binance kártya saját kivonatának oszlopai nincsenek nyilvánosan dokumentálva, ezért ezeket még nem olvassa be – ha elküldöd egy (kitakart) mintádat, bekerülhet. Addig a „Fizetés kriptóval” típussal kézzel rögzíthetők. (A Binance kártya az EGT-ben 2023. december 20-án megszűnt.)
- Minden napi tétel mögé elmenti az **eredeti sorokat** és a forrásfájl nevét és SHA-256 ujjlenyomatát (API-nál a lekérés idejét); újraimportáláskor ezek is az új forráséra cserélődnek, a tétel törlésekor törlődnek.
