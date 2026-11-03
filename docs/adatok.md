# Adatok tárolása, mentés, export

[← vissza a README-hez](../README.md)

Minden adat helyben, profilonként egy SQLite adatbázisfájlban van, az app nem küld adatot sehova. (Az adatmappa neve a Kriptomérleg átnevezés után is `hu.kriptoado.app` maradt, hogy a korábban rögzített adatok megmaradjanak.) Hálózati forgalom: az MNB árfolyam lekérése, a frissítéskeresés (kikapcsolható), és csak ha kéred, a tőzsdei API-lekérés (Binance, Bybit, Kraken).

| Rendszer | Hely |
|---|---|
| Windows | `%APPDATA%\hu.kriptoado.app\kripto-ado.sqlite` (pl. `C:\Users\<név>\AppData\Roaming\hu.kriptoado.app\`) |
| Linux / NixOS | `~/.local/share/hu.kriptoado.app/kripto-ado.sqlite` (ill. `$XDG_DATA_HOME/hu.kriptoado.app/`) |

Az adatmappa tartalma:

| Fájl / mappa | Tartalom |
|---|---|
| `kripto-ado.sqlite` | a *Saját* (alapértelmezett) profil adatbázisa |
| `profiles/<azonosító>.sqlite` | a további profilok adatbázisai |
| `profiles.json` | profilazonosító → megjelenített név |
| `current-profile` | az utoljára használt profil azonosítója |
| `backups/` | automatikus mentések (`<profil>-ÉÉÉÉ-HH-NN.sqlite`, hetente, az utolsó 8 marad), visszaállítás előtti (`<profil>-visszaallitas-elott-…`) és törölt profilok (`<profil>-torolt-profil-…`) mentései |

Ugyanebben a mappában a beépített böngészőmotor (WebView) gyorsítótára is megtalálható, ezek törölhetők.

**Táblák:**

`transactions` – a rögzített tételek

| Oszlop | Típus | Jelentés |
|---|---|---|
| `id` | INTEGER | azonosító |
| `date` | TEXT | teljesítés napja, `ÉÉÉÉ-HH-NN` |
| `currency` | TEXT | `EUR`, `HUF`, `USD`, … |
| `amount` | REAL | összeg a devizában; pozitív = kiadás (befizetés, vétel), negatív = bevétel (kivétel, eladás) |
| `rate` | REAL | Ft / egység (HUF esetén 1) |
| `rate_source` | TEXT | `MNB`, `kézi` vagy `-` (forintos tétel) |
| `rate_date` | TEXT | az MNB árfolyam közzétételének napja (hétvégén az előző munkanap) |
| `huf` | INTEGER | forintösszeg (`amount × rate`, egészre kerekítve) – az adószámítás ebből dolgozik |
| `note` | TEXT | megjegyzés (importnál pl. „Binance · 3 vétel · díj 1,20 EUR”) |
| `source` | TEXT | `kézi`, `excel`, `binance`, `revolut`, `kraken`, `bybit` |
| `kind` | TEXT | `deposit` / `withdrawal` / `payment` / `income` (kézi; `payment` = kriptóval fizetett áru, szolgáltatás; `income` = kriptóban kapott bér, ellenérték), `buy` / `sell` (tőzsdei import) |
| `fee` | REAL | ebből díj a tétel devizájában (tájékoztató) |
| `trades` | INTEGER | hány ügyletből áll a napi tétel (kézinél 1; a kisértékű ügylet szabályához) |
| `external_id` | TEXT | importált napi tétel kulcsa (pl. `binance:2026-03-04:EUR:buy`), ez akadályozza meg a duplikálást |
| `created_at` | TEXT | rögzítés ideje |

`rates` – az MNB-től már lekért árfolyamok gyorsítótára (`currency`, `date`, `rate`), hogy ugyanazt a napot ne kelljen újra lekérni. Hétvégére és ünnepnapra nincs MNB árfolyam, ezért a már feloldott múltbeli napokat a program futás közben a memóriában is megjegyzi (a mai napot nem, mert az MNB napközben tesz közzé); az Áttekintés euróárfolyamát naponta egyszer kéri le.

`import_details` – ügyletszintű nyomkövetés: az importált napi tételek mögötti eredeti sorok (`external_id` – a tétel kulcsa, `origin` – fájlnév és SHA-256, vagy API és lekérési idő, `raw` – az eredeti sor pontosvesszővel összefűzve, API-nál a rekord fő mezői). A tétel törlésekor egy trigger ezeket is törli.

`settings` – kulcs–érték beállítások (`key`, `value`): `kiserteku` (`on`/`off`, kisértékű ügyletek), `reminders` (`off` = nincs emlékeztető), `emlekezteto:<év>:<szakasz>` (mikor ment ki az adott rendszerértesítés).

`loss_declarations` – évenként (`year`) a felhasználó jelölése (`declared`: 1/0), hogy az év veszteségét feltüntette-e az SZJA-bevallásban. Ha egy évhez nincs sor, az alapértelmezést lásd [Az adószámítás szabályai](adoszamitas.md).

Az éves összesítő és az adó nincs eltárolva, mindig a tételekből számolódik. Az adatbázis sémaverzióját a `PRAGMA user_version` tárolja; régebbi adatbázist az app induláskor automatikusan frissít.

**Mentés:** az *Adatok és mentés* oldalon. A *Mentés fájlba* a teljes adatbázis pillanatképét írja ki (`.sqlite`); jelszóval titkosítva `.kmbackup` fájl lesz (formátum: `KMBACKUP1` fejléc + 16 bájt só + 12 bájt nonce + ChaCha20-Poly1305 titkosított adatbázis, a kulcs Argon2id-vel a jelszóból). A jelszót az app nem tárolja. A *Visszaállítás* előtt a jelenlegi állapotról automatikusan mentés készül. Az app induláskor és profilváltáskor hetente magától is ment a `backups/` mappába. Kézzel is elég a `.sqlite` fájlt átmásolni (az app bezárt állapotában).

**ÁNYK XML:** az ÁNYK importálható XML-formátuma (`<nyomtatvanyok>`, `<mezo eazon="…">`) ismert, de a 25SZJA kriptós mezőinek kódjai (164., 202–209. sor) nincsenek nyilvánosan publikálva, évente változhatnak, és a NAV az ÁNYK-t 2026. december 31-ével kivezeti (utódja az ONYA, amelynek XML-importja még nincs dokumentálva). Ezért az app most nem készít XML-t; helyette a Bevallási segéd a kitöltendő sorokat CSV-be menti.

**Export:** Excel (.xlsx, „Tételek” és „Éves összesítő” lap, számként tárolt összegekkel, szűrővel; ha van importált tétel, „Eredeti sorok” lap is) és CSV (pontosvessző, tizedesvessző, UTF-8 BOM – Excelben közvetlenül megnyitható).
