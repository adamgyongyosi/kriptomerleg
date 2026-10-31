# Példa importfájlok

Kitalált adatokkal kitöltött minták minden támogatott formátumhoz. Az appban az *Importálás* gombbal (bal alsó sarok) bármelyik beolvasható, így megnézheted, hogyan működik az import, és összevetheted a saját exportod szerkezetével.

> Tipp: a kipróbálás után a *Tételek* oldalon a forrásszűrővel (pl. „Binance”) egy gombnyomással törölheted a példatételeket.

| Fájl | Formátum | Mit mutat be | Importált napi tétel |
|---|---|---|---|
| [`binance-transaction-history.csv`](binance-transaction-history.csv) | Binance – Wallet → Transaction History → Export | EUR-ért vett BTC külön díjsorral; Convert EUR → ETH; USDT → SOL csere (kimarad); eladás EUR-ra díjjal; be- és kifizetés (kimarad) | 4 |
| [`kraken-ledgers.csv`](kraken-ledgers.csv) | Kraken – History → Export → Ledgers | ZEUR → BTC vétel díjjal; ETH → BTC csere (kimarad); BTC → ZEUR eladás; be- és kifizetés (kimarad) | 2 |
| [`revolut-crypto-statement.csv`](revolut-crypto-statement.csv) | Revolut – kripto kivonat (újabb) | `Buy` / `Sell` sorok €-s értékkel és díjjal; `Send` és jutalom sorok (kimaradnak) | 4 |
| [`revolut-crypto-statement-regi.csv`](revolut-crypto-statement-regi.csv) | Revolut – kripto kivonat (régebbi) | `EXCHANGE` sorok „Fiat amount (inc. fees)” oszloppal; `TRANSFER` és `DECLINED` sor (kimarad) | 3 |
| [`bybit-spot-trade-history.csv`](bybit-spot-trade-history.csv) | Bybit – Orders → Spot Order → Trade History | BTCEUR / ETHEUR vétel kriptóban fizetett díjjal; BTCUSDT (kimarad); eladás EUR díjjal | 3 |
| [`bybit-convert-history.csv`](bybit-convert-history.csv) | Bybit – Convert history | EUR → BTC, ETH → EUR; USDT → BTC (kimarad) | 2 |
| [`bank-kivonat.csv`](bank-kivonat.csv) | Banki kivonat (Revolut számla) | „To BINANCE”, „Transfer to Kraken” kiutalás (befizetés), „Payment from Payward” (Kraken) jóváírás (kivétel); kártyás vásárlás, feltöltés és visszafordított utalás (kimarad) | 3 |
| [`sajat-excel-nyilvantartas.xlsx`](sajat-excel-nyilvantartas.xlsx) | Saját, régi Excel (Importálás → „Excel import”) | Évszám nevű lapok (2025, 2026), A oszlop dátum, B oszlop forint; negatív = kivétel; a „Leírás” lap kimarad | 9 |

## A saját fájlod szerkezete

- Az app az **oszlopnevek** alapján ismeri fel a formátumot, a sorrend nem számít. A kis- és nagybetű, a szóköz és az írásjelek sem.
- CSV-nél a vessző, a pontosvessző és a tab elválasztó is működik. Excel (.xlsx) esetén az első munkalapot olvassa.
- Ha a fájl előtt néhány sor leírás van, azt átugorja: a fejléc az első olyan sor, amelyben legalább 3 kitöltött cella van.
- Összegek: `1,234.56`, `1 234,56`, `€12.00`, `-€5`, `1.5E-7` is jó.
- Dátumok: `2026-01-13 09:20:03`, `2026-01-13T09:20:03Z`, `13/01/2026 09:20`, `Jan 5, 2026, 10:23:45 AM` stb. A tőzsdék UTC idejét magyar időre váltja.
- Ha a fájlt nem ismeri fel, a hibaüzenet kiírja a talált oszlopneveket. Ezt érdemes elküldeni, hogy a formátum bekerülhessen.

A mintafájlokat egy automatikus teszt is beolvassa (`src-tauri/src/import/mod.rs` → `example_files_import`), így mindig működőképesek maradnak.
