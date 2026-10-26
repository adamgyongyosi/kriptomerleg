# Kripto adó

Asztali alkalmazás (Windows, Linux) a kriptóba befizetett és onnan kivett összegek nyilvántartására és a magyar kriptó-SZJA kiszámítására.

- Gyors rögzítés: Befizetés / Kivétel kapcsoló, € / Ft váltó (alapértelmezés: EUR, minden mentés után visszaáll), és élőben látszik, mennyivel változik az évi adó. Mínusz jellel beírt összeg automatikusan kivétel.
- EUR tételeknél az MNB hivatalos árfolyamát automatikusan lekéri a teljesítés napjára (hétvégén/ünnepnap az utolsó közzétettet); kattintásra kézi (pl. banki) árfolyam is megadható. Forintos tétel is rögzíthető.
- Éves eredmény = kivételek − befizetések; nyereség után 15% SZJA.
- Adókiegyenlítés: a veszteség 15%-a levonható, legfeljebb a tárgyévi és az előző két évi kriptoadó erejéig; a 2025-ös bevallástól a veszteség nem évül el (2025. évi LXXXIII. tv.).
- Bevallási segéd: mit kell a 25SZJA 164. sorába és a 06-os lap 202–209. sorába írni (NAV kitöltési útmutató alapján).
- **Tőzsdei import:** Binance, Revolut, Kraken és Bybit export (CSV/Excel) beolvasása – a fiat ↔ kripto ügyletek díjakkal együtt, naponta összevonva, MNB árfolyamon.
- Régi Excel import (évszám nevű lapok, A: dátum, B: forint), CSV export.
- Adatok helyben, SQLite-ban.

> Tájékoztató számítás, nem adótanácsadás.

## Letöltés

Minden `main`-re pusholt változásból automatikusan új **[Release](https://github.com/adamgyongyosi/kripto-ado/releases)** készül (`v<verzió>-build.<szám>`), egy `v*` tagből pedig verziózott kiadás. A release-ben:

| Fájl | Rendszer |
|---|---|
| `kripto-ado_<verzió>_windows_x64-setup.exe` | Windows telepítő |
| `kripto-ado_<verzió>_windows_x64-portable.exe` | Windows, telepítés nélkül |
| `kripto-ado_<verzió>_linux_amd64.deb` | Debian / Ubuntu: `sudo apt install ./kripto-ado_*.deb` |
| `kripto-ado_<verzió>_linux_amd64.AppImage` | bármely Linux: `chmod +x`, majd futtatás |

NixOS-re lásd lent.

## Tőzsdei import

*Importálás* (bal alsó sarok) → export fájl kiválasztása → előnézet → *Importálás*.

| Tőzsde | Honnan töltsd le | Mit olvas be |
|---|---|---|
| Binance | Wallet → Transaction History → Export (Generate all statements) | A fiat egyenleg minden ügyletből eredő változása (Transaction Spend/Revenue/Fee, Convert, Buy/Sell…) |
| Revolut | Kripto → … → Statement (kivonat), CSV vagy Excel | `Buy` / `Sell` sorok értéke és díja (újabb formátum); `EXCHANGE` sorok fiat összege (régi formátum) |
| Kraken | History → Export → Ledgers | `trade` / `spend` / `receive` sorok fiat összege, díj levonva |
| Bybit | Orders → Spot Order → Trade History → Export; Convert → History → Export | Fiat párok (pl. BTCEUR) kereskedései díjjal; fiat ↔ kripto konvertálások |

A formátumot az oszlopnevek alapján ismeri fel. A menük neve a tőzsdei appok frissítésével változhat.

**Példafájlok:** az [`examples/`](examples/) mappában minden formátumhoz van egy kitalált adatokkal kitöltött, importálható minta (és a régi Excel nyilvántartás mintája is). A [`examples/README.md`](examples/README.md) leírja, melyik fájl mit mutat be, és milyen szerkezetű lehet a saját exportod.

**Hogyan számol:**

- Csak a **fiat ↔ kripto** ügyletek számítanak: vétel fiatért = kiadás, eladás fiatra = bevétel. A tőzsdére utalt vagy onnan kivett pénz, és a kripto–kripto csere (pl. BTC → USDT) nem adóköteles, ezeket kihagyja.
- A **díjak** a kiadást növelik, illetve a bevételt csökkentik. Ha a díj kriptóban van (pl. Bybit vételnél), az ügylet árfolyamán számítja át.
- Az ügyleteket **naponta, devizánként és irányonként összevonja** egy tételbe; a forintra váltás az aznapi MNB árfolyammal történik (USD-t, GBP-t stb. is kezel).
- A tőzsdék UTC-ben adják az időt, az app ezt magyar időre (CET/CEST) váltja a nap meghatározásához.
- **Újraimportálás** (pl. hosszabb időszakkal) nem duplikál: a már meglévő napi tételeket frissíti.
- Ha az adott évben **kézi vagy Excel-tétel** is van, figyelmeztet: ha azok ugyanazokat a vásárlásokat jelentik (pl. a tőzsdére utalt euró), töröld őket, különben a kiadás duplán számít.
- A *Tételek* oldalon forrás szerint szűrhetsz, és egy forrás adott évi tételeit egyben törölheted.
- Figyelmeztet, ha olyan sort talál, amit kézzel kell rögzítened (pl. Binance kártyás vásárlás fiat összeg nélkül, P2P).

## Adatok tárolása

Minden adat helyben, egyetlen SQLite adatbázisfájlban van (`kripto-ado.sqlite`), az app nem küld adatot sehova. Az egyetlen hálózati forgalom az MNB árfolyam lekérése.

| Rendszer | Hely |
|---|---|
| Windows | `%APPDATA%\hu.kriptoado.app\kripto-ado.sqlite` (pl. `C:\Users\<név>\AppData\Roaming\hu.kriptoado.app\`) |
| Linux / NixOS | `~/.local/share/hu.kriptoado.app/kripto-ado.sqlite` (ill. `$XDG_DATA_HOME/hu.kriptoado.app/`) |

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
| `kind` | TEXT | `deposit` / `withdrawal` (kézi), `buy` / `sell` (tőzsdei import) |
| `fee` | REAL | ebből díj a tétel devizájában (tájékoztató) |
| `external_id` | TEXT | importált napi tétel kulcsa (pl. `binance:2026-03-04:EUR:buy`), ez akadályozza meg a duplikálást |
| `created_at` | TEXT | rögzítés ideje |

`rates` – az MNB-től már lekért árfolyamok gyorsítótára (`currency`, `date`, `rate`), hogy ugyanazt a napot ne kelljen újra lekérni.

Az éves összesítő és az adó nincs eltárolva, mindig a tételekből számolódik. Az adatbázis sémaverzióját a `PRAGMA user_version` tárolja; régebbi adatbázist az app induláskor automatikusan frissít.

**Mentés:** elég a `kripto-ado.sqlite` fájlt átmásolni (az app bezárt állapotában). Visszaállításhoz tedd vissza ugyanoda. Ember által olvasható formában a *Export CSV* gomb ad ki mindent (pontosvessző, tizedesvessző, UTF-8 – Excelben közvetlenül megnyitható). A fájl bármilyen SQLite eszközzel (pl. DB Browser for SQLite) is megnyitható.

## Fejlesztés

Kell: Node 20+, Rust (stable), Windowson MSVC C++ build tools, Linuxon `libwebkit2gtk-4.1-dev` és társai.

```bash
npm install
npm run tauri dev
```

Tesztek:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Build

- Windows + Linux egyszerre (Windowson, a Linux rész WSL-ben): `powershell -ExecutionPolicy Bypass -File scripts\build-all.ps1`
- Csak Linux: `bash scripts/build-linux.sh`
- GitHub Actions (`.github/workflows/build.yml`): minden pushnál Windows (NSIS + hordozható exe) és Linux (.deb, AppImage) build és Rust tesztek, `nix build` ellenőrzés, majd `main`-re pusholáskor automatikus GitHub Release a fájlokkal.
- Verzió emelése: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` és `nix/package.nix` `version` mezője; kiadás taggel: `git tag v0.2.0 && git push --tags`.

A kimenetek a `telepito/windows` és `telepito/linux` mappába kerülnek.

Linux függőségek (Debian/Ubuntu):

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

## NixOS

A repóban van egy Nix flake (`flake.nix`, `nix/package.nix`), ebből a NixOS natívan fordítja az appot.

Futtatás telepítés nélkül (a repó privát, ezért SSH-n keresztül):

```bash
nix run git+ssh://git@github.com/adamgyongyosi/kripto-ado
```

Rendszerszintű telepítés flake-alapú NixOS konfigurációban:

```nix
# flake.nix
inputs.kripto-ado.url = "git+ssh://git@github.com/adamgyongyosi/kripto-ado";

# configuration.nix (a modulnak átadott inputs-szal)
environment.systemPackages = [ inputs.kripto-ado.packages.${pkgs.stdenv.hostPlatform.system}.default ];
```

Fejlesztői környezet: `nix develop`, utána `npm install && npm run tauri dev`.

Alternatíva: az AppImage NixOS-en `appimage-run`-nal indítható (`programs.appimage.enable = true;`).

## Felépítés

- `src-tauri/src/tax.rs` – adószámítás és adókiegyenlítés (unit tesztekkel)
- `src-tauri/src/mnb.rs` – MNB SOAP árfolyam-lekérés
- `src-tauri/src/excel.rs` – régi Excel import
- `src-tauri/src/import/` – tőzsdei import: közös CSV/Excel olvasó, szám- és dátumértelmezés (`mod.rs`), formátumonként `binance.rs`, `revolut.rs`, `kraken.rs`, `bybit.rs` (mintafájlos tesztekkel)
- `src-tauri/src/lib.rs` – SQLite tárolás, Tauri parancsok, CSV export
- `src/` – React felület (`App.tsx`, `EntryForm.tsx`, `ImportDialog.tsx`, `Filing.tsx`, `Chart.tsx`)
