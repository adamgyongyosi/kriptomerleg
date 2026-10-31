<p align="center"><img src="brand/logo-mark.svg" width="96" alt="Kriptomérleg logó"></p>

# Kriptomérleg

**A kriptóadód, a saját gépeden.** Asztali alkalmazás (Windows, Linux, NixOS) a kriptóba befizetett és onnan kivett összegek nyilvántartására és a magyar kriptó-SZJA kiszámítására – fiók és felhő nélkül, minden adat helyben marad.

Weboldal: <https://adamgyongyosi.github.io/kriptomerleg/> (a `site/` mappából, lásd [Weboldal](#weboldal)). Repó: <https://github.com/adamgyongyosi/kriptomerleg> (2026. október 31-ig `kripto-ado` volt a neve; a GitHub a régi címekről átirányít).

- Gyors rögzítés: Befizetés / Kivétel / Fizetés kriptóval kapcsoló, € / Ft váltó és a „…” menüben USD, GBP, CHF (alapértelmezés: EUR, minden mentés után visszaáll), és élőben látszik, mennyivel változik az évi adó. Mínusz jellel beírt összeg automatikusan kivétel. Az összeg mező gépelés közben ezres tagolást kap (`899 100`); `1.000.000` és `1.000,50` alakban beillesztve is jól értelmezi.
- Devizás (EUR, USD, GBP, CHF) tételeknél az MNB hivatalos árfolyamát automatikusan lekéri a teljesítés napjára (hétvégén/ünnepnap az utolsó közzétettet); kattintásra kézi (pl. banki) árfolyam is megadható. Forintos tétel is rögzíthető.
- Éves eredmény = kivételek − befizetések; nyereség után 15% SZJA.
- **Fizetés kriptóval:** ha kriptóval fizetsz árut vagy szolgáltatást (ajándékkártya, utazás), annak forintértéke bevételnek számít, mint egy kivétel. Külön tételtípus („Fizetés kriptóval”), a CSV-exportban is.
- **Adómentesen még kivehető:** az Áttekintésen látszik, mekkora kivételig (vagy kriptós fizetésig) nem keletkezik adó az évben, a folyó évben euróban is, a mai MNB árfolyamon.
- Adókiegyenlítés: a veszteség 15%-a levonható, legfeljebb a tárgyévi és az előző két évi kriptoadó erejéig; a 2025-ös bevallástól a veszteség nem évül el (2025. évi LXXXIII. tv.). Korábbi év vesztesége csak akkor, ha azt annak az évnek a bevallásában feltüntették – ezt évenként jelölni kell (lásd [Az adószámítás szabályai](#az-adószámítás-szabályai)).
- Bevallási segéd: mit kell a 25SZJA 164. sorába és a 06-os lap 202–209. sorába írni (NAV kitöltési útmutató alapján). A kitöltendő sorok CSV-be is menthetők (nyomtatvány, lap, sor, oszlop, megnevezés, érték).
- **Éves nyilvántartás PDF-ben** (Szja tv. 67/C. §): összesítő, veszteség-nyilvántartás (67/C. § (7a)) és az év összes tétele MNB árfolyammal, oldalszámozva – archiváláshoz, ellenőrzéshez. A betűkészlet (DejaVu Sans, `src-tauri/fonts/`, licenc: `DEJAVU-LICENSE.txt`) a programba van fordítva.
- **Tőzsdei import:** Binance, Revolut, Kraken és Bybit export (CSV/Excel) beolvasása – a fiat ↔ kripto ügyletek díjakkal együtt, naponta összevonva, MNB árfolyamon. Ugyanaz a fájl újra importálva frissít, nem duplikál; ha egy nap már szerepel más forrásból (azonos nap, irány, legfeljebb 1% eltérés), az előnézet „lehet dupla” jelölést ad, és alapból kihagyja.
- **Kisértékű ügyletek** (67/C. § (2)) kezelése bekapcsolható: a minimálbér 10%-áig terjedő, napi egyetlen bevétel kihagyható az eredményből (lásd lent).
- **Önellenőrzés-segéd:** ha egy korábbi év vesztesége kimaradt a bevallásból, a Bevallási segéd megmutatja, melyik nyomtatvány melyik sorába kell pótolni, meddig, és mennyivel csökkenne utána az adó.
- **Határidő-emlékeztető:** március 15. (eSZJA tervezet) és május 20. között az app jelzi az előző évi kriptoadót, adókiegyenlítést vagy bevallandó veszteséget; szakaszonként egyszer rendszerértesítést is küld (kikapcsolható).
- **Adatok és mentés** oldal: Excel (.xlsx) és CSV export, mentés fájlba (opcionálisan jelszóval titkosítva), visszaállítás, heti automatikus mentés, profilok.
- **Több profil:** külön nyilvántartás pl. házastársnak, családtagnak; profilváltó a bal oldali sávban.
- **Automatikus frissítés:** induláskor megnézi, van-e újabb kiadás, és felajánlja a letöltést és az újraindítást (Windows, AppImage). Kikapcsolható az *Adatok és mentés* oldalon.
- Régi Excel import (évszám nevű lapok, A: dátum, B: forint).
- Adatok helyben, SQLite-ban.

> Tájékoztató számítás, nem adótanácsadás.

## Az adószámítás szabályai

A számítás az Szja tv. 67/C. §-án és a NAV tájékoztatóin alapul:

1. **Éves eredmény** = az év kivételei (bevétel) − az év befizetései (kriptóra fordított kiadás), forintban. Nyereség után 15% SZJA, határidő a következő év május 20.
2. **2022 előtti tételek:** a 15%-os kriptoszabály 2022-től él. A korábban nem adózott ügyletek a NAV szerint a **2022-es eredménybe** számítanak, ezért az app a 2021-es (és korábbi) befizetéseket 2022-höz adja; a 2021-es évnek nincs külön eredménye.
3. **Adókiegyenlítés** (67/C. § (5)–(7)): a veszteség 15%-a érvényesíthető, legfeljebb a tárgyévi és az előző két évben bevallott kriptoadó erejéig, csökkentve az előző két évben már érvényesített kiegyenlítéssel (25SZJA-06 lap 202–209. sor → C lap 75. sor).
4. **Csak a bevallott veszteség vonható le később.** A NAV szerint az adókiegyenlítés csak akkor alkalmazható, ha a veszteséget a keletkezés évének bevallásában (164. sor „a” oszlop) feltüntették. Az *Évek és adó* oldal *Bevallva* oszlopában, illetve az *Áttekintés* figyelmeztetésében évenként jelölhető, melyik évét vallottad be. Ha nincs jelölve:
   - lejárt határidejű évnél (pl. a 2024-es veszteség, határidő 2025. május 20.) az app **bevallatlannak** veszi, és nem vonja le;
   - a még be nem adott évnél (a folyó év) bevallottnak veszi, mert a bevallási segéd szerint fogod kitölteni.

   Ha egy korábbi év vesztesége kimaradt, az adott évi nyomtatványon pótolható (az elévülésig, 5 évig), utána jelöld be. Az *önellenőrzés-segéd* (Bevallási segéd oldal) megmutatja a nyomtatványt (pl. 24SZJA), a 164. sor „a” oszlopába írandó összeget és a várható adócsökkenést. Nyitott kérdés, hogy ez önellenőrzés („O”) vagy helyesbítés („H”): a veszteség a fizetendő adót nem változtatja, ezért az útmutató szerint inkább helyesbítés – ezt a NAV-val (1819) érdemes egyeztetni.
5. **Kisértékű ügylet** (67/C. § (2)): nem kell bevételt megállapítani abból az ügyletből, amelynek bevétele legfeljebb a minimálbér 10%-a, ha aznap nem volt más bevételt hozó ügylet, és az ilyen bevételek éves összege nem haladja meg a minimálbért. A minimálbér az év első napján érvényes havi összeg: 2022: 200 000, 2023: 232 000, 2024: 266 800, 2025: 290 800, 2026: 322 800 Ft (a 2026-os érték sajtóforrásból). Ismeretlen évre az app nem alkalmaz mentességet. Ha az éves összeg túllépi a minimálbért, egyik ilyen bevétel sem mentes. A vételek (kiadások) ettől még teljes összegben számítanak. A beállítás alapból ki van kapcsolva (*Évek és adó* oldal); importált tételnél a napi összevont sor ügyletszáma dönt.
6. **Időkorlát:** a 2025-ös bevallástól (2025. évi LXXXIII. tv.) bármely korábbi év bevallott, még fel nem használt vesztesége figyelembe vehető; a régi kétéves szabály a 2022-től kezdődő évekre ugyanezt adja.

Példa: 2022–2025 között csak befizetések (összesen 18 027 000 Ft veszteség), 2026-ban 24 021 003 Ft nyereség. Bruttó adó 3 603 150 Ft. Ha mind a négy év veszteségét bevallottad, 2 704 050 Ft adókiegyenlítés jár, és 899 100 Ft marad fizetendő; ha egyiket sem, 3 603 150 Ft a fizetendő.

## Letöltés

Minden `main`-re pusholt változásból automatikusan új **[Release](https://github.com/adamgyongyosi/kriptomerleg/releases)** készül saját verziószámmal: a `package.json` fő- és alverziója, harmadik tagként a CI futás sorszáma (pl. `v0.3.12`, „Kriptomérleg 0.3.12”). Ez a verzió kerül az appba is, így az automatikus frissítő minden újabb kiadást felajánl. Egy `v*` tag (pl. `v1.0.0`) pontosan azt a verziót adja ki. A release-ben:

| Fájl | Rendszer |
|---|---|
| `kriptomerleg_windows_x64-setup.exe` | Windows telepítő |
| `kriptomerleg_windows_x64-portable.exe` | Windows, telepítés nélkül |
| `kriptomerleg_linux_amd64.deb` | Debian / Ubuntu: `sudo apt install ./kriptomerleg_linux_amd64.deb` |
| `kriptomerleg_linux_amd64.AppImage` | bármely Linux: `chmod +x`, majd futtatás |

A fájlnevekben nincs verziószám, így a `https://github.com/adamgyongyosi/kriptomerleg/releases/latest/download/<fájlnév>` link mindig a legfrissebbre mutat (a weboldal letöltőgombjai is ezt használják). A verzió a release nevében és tagjében szerepel.

NixOS-re lásd lent.

## Tőzsdei import

*Importálás* (bal alsó sarok) → export fájl kiválasztása → előnézet → *Importálás*.

| Tőzsde | Honnan töltsd le | Mit olvas be |
|---|---|---|
| Binance | Wallet → Transaction History → Export (Generate all statements) | A fiat egyenleg minden ügyletből eredő változása (Transaction Spend/Revenue/Fee, Convert, Buy/Sell…) |
| Revolut | Kripto → … → Statement (kivonat), CSV vagy Excel | `Buy` / `Sell` sorok értéke és díja (újabb formátum); `EXCHANGE` sorok fiat összege (régi formátum) |
| Kraken | History → Export → Ledgers | `trade` / `spend` / `receive` sorok fiat összege, díj levonva |
| Bybit | Orders → Spot Order → Trade History → Export; Convert → History → Export | Fiat párok (pl. BTCEUR) kereskedései díjjal; fiat ↔ kripto konvertálások |
| Bank (Revolut, Wise, magyar bankok) | a netbank számlakivonat-exportja (CSV/Excel) | A kriptotőzsdékre (Binance, Kraken/Payward, Bybit, Coinbase, Bitstamp, Bitpanda, OKX, KuCoin, Crypto.com, …) küldött utalás = befizetés, onnan érkező = kivétel. Közelítés: ha van tőzsdei export, az pontosabb. |

**Kraken API:** az Importálás ablakban a „Kraken: lekérés közvetlenül, API kulccsal” a főkönyvet (Ledgers) a Kraken API-ján kéri le, és ugyanúgy dolgozza fel, mint a Ledgers exportot. A kulcshoz csak „Query Ledger Entries” jog kell; a kulcsot és a titkot az app nem menti. A lekérés oldalanként 50 sort ad, a Kraken lekérési korlátja miatt oldalanként kb. 2,5 mp. A többi tőzsde API-ja (Binance, Bybit) még nincs bekötve.

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

Minden adat helyben, profilonként egy SQLite adatbázisfájlban van, az app nem küld adatot sehova. (Az adatmappa neve a Kriptomérleg átnevezés után is `hu.kriptoado.app` maradt, hogy a korábban rögzített adatok megmaradjanak.) Az egyetlen hálózati forgalom az MNB árfolyam lekérése.

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
| `kind` | TEXT | `deposit` / `withdrawal` / `payment` (kézi; `payment` = kriptóval fizetett áru, szolgáltatás), `buy` / `sell` (tőzsdei import) |
| `fee` | REAL | ebből díj a tétel devizájában (tájékoztató) |
| `trades` | INTEGER | hány ügyletből áll a napi tétel (kézinél 1; a kisértékű ügylet szabályához) |
| `external_id` | TEXT | importált napi tétel kulcsa (pl. `binance:2026-03-04:EUR:buy`), ez akadályozza meg a duplikálást |
| `created_at` | TEXT | rögzítés ideje |

`rates` – az MNB-től már lekért árfolyamok gyorsítótára (`currency`, `date`, `rate`), hogy ugyanazt a napot ne kelljen újra lekérni.

`settings` – kulcs–érték beállítások (`key`, `value`): `kiserteku` (`on`/`off`, kisértékű ügyletek), `reminders` (`off` = nincs emlékeztető), `emlekezteto:<év>:<szakasz>` (mikor ment ki az adott rendszerértesítés).

`loss_declarations` – évenként (`year`) a felhasználó jelölése (`declared`: 1/0), hogy az év veszteségét feltüntette-e az SZJA-bevallásban. Ha egy évhez nincs sor, az alapértelmezést lásd [fent](#az-adószámítás-szabályai).

Az éves összesítő és az adó nincs eltárolva, mindig a tételekből számolódik. Az adatbázis sémaverzióját a `PRAGMA user_version` tárolja; régebbi adatbázist az app induláskor automatikusan frissít.

**Mentés:** az *Adatok és mentés* oldalon. A *Mentés fájlba* a teljes adatbázis pillanatképét írja ki (`.sqlite`); jelszóval titkosítva `.kmbackup` fájl lesz (formátum: `KMBACKUP1` fejléc + 16 bájt só + 12 bájt nonce + ChaCha20-Poly1305 titkosított adatbázis, a kulcs Argon2id-vel a jelszóból). A jelszót az app nem tárolja. A *Visszaállítás* előtt a jelenlegi állapotról automatikusan mentés készül. Az app induláskor és profilváltáskor hetente magától is ment a `backups/` mappába. Kézzel is elég a `.sqlite` fájlt átmásolni (az app bezárt állapotában).

**ÁNYK XML:** az ÁNYK importálható XML-formátuma (`<nyomtatvanyok>`, `<mezo eazon="…">`) ismert, de a 25SZJA kriptós mezőinek kódjai (164., 202–209. sor) nincsenek nyilvánosan publikálva, évente változhatnak, és a NAV az ÁNYK-t 2026. december 31-ével kivezeti (utódja az ONYA, amelynek XML-importja még nincs dokumentálva). Ezért az app most nem készít XML-t; helyette a Bevallási segéd a kitöltendő sorokat CSV-be menti.

**Export:** Excel (.xlsx, „Tételek” és „Éves összesítő” lap, számként tárolt összegekkel, szűrővel) és CSV (pontosvessző, tizedesvessző, UTF-8 BOM – Excelben közvetlenül megnyitható).

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

```bash
npm test
```

A `npm test` a felület-teszteket futtatja (Vitest + Testing Library, jsdom): összegformázás, határidő-emlékeztető, a rögzítő űrlap (deviza, kriptós fizetés, MNB árfolyam), appon belüli párbeszédablak. A Tauri hívásokat a `@tauri-apps/api/mocks` helyettesíti, így asztali app nélkül is futnak. A CI mindkettőt lefuttatja.

## Build

- Helyi build: csak a Windows hordozható exe – `npm run tauri build -- --no-bundle`; a kimenet `<CARGO_TARGET_DIR>\release\kriptomerleg.exe`, ezt másold `telepito\windows\kriptomerleg_<verzió>_windows_x64-portable.exe` néven. A telepítőt, a .deb-et és az AppImage-et a GitHub Actions készíti minden pushnál.
- Windows + Linux egyszerre, ha mégis helyben kell (Windowson, a Linux rész WSL-ben): `powershell -ExecutionPolicy Bypass -File scripts\build-all.ps1`
- Csak Linux: `bash scripts/build-linux.sh`
- GitHub Actions (`.github/workflows/build.yml`): minden pushnál Windows (NSIS + hordozható exe) és Linux (.deb, AppImage) build és Rust tesztek, `nix build` ellenőrzés, majd `main`-re pusholáskor automatikus GitHub Release a fájlokkal.
- Verzió emelése: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `nix/package.nix`, valamint a `packaging/` fájljaiban; kiadás taggel: `git tag v0.3.0 && git push --tags`. Az automatikus frissítő csak akkor ajánl frissítést, ha a verziószám nő.

### Automatikus frissítés (updater)

- Az app a `https://github.com/adamgyongyosi/kriptomerleg/releases/latest/download/latest.json` fájlból tudja meg a legújabb verziót; a csomagokat a `tauri.conf.json`-ban lévő nyilvános kulccsal ellenőrzi.
- Az aláíráshoz a CI-nek két titok kell (*Settings → Secrets and variables → Actions*): `TAURI_SIGNING_PRIVATE_KEY` (a privát kulcsfájl tartalma) és `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (üres, ha a kulcsnak nincs jelszava). Ha nincsenek beállítva, a build ugyanúgy elkészül, csak `latest.json` és `.sig` fájlok nélkül.
- Új kulcspár: `npx tauri signer generate -w ~/.tauri/kriptomerleg-updater.key`, majd a `.pub` tartalma a `tauri.conf.json` `plugins.updater.pubkey` mezőjébe. A privát kulcs ne kerüljön a repóba.
- A frissítés csak publikus repóból működik (privát repó kiadásai bejelentkezés nélkül nem tölthetők le); ugyanez igaz a weboldal letöltőgombjaira.
- A 2026. október 31. előtt készült példányok (a 0.3.13 előtti kiadások és a korábbi helyi buildek) még a lecserélt kulcs nyilvános részét tartalmazzák, ezért „The signature was created with a different key” hibát adnak. Ezeket egyszer kézzel kell frissíteni a legújabb kiadásra; onnantól az automatikus frissítés működik. Az app ilyenkor ezt írja ki.

### Linux csomagtárak

A `packaging/` mappában előkészített, még be nem küldött csomagleírások:

| Fájl | Hová | Teendő beküldés előtt |
|---|---|---|
| `packaging/aur/PKGBUILD` | AUR (`kriptomerleg-bin`, a release .deb fájljából) | publikus repó, `v<verzió>` tag, licenc, `updpkgsums` |
| `packaging/flatpak/hu.kriptoado.app.yml` + `.metainfo.xml` | Flathub | publikus repó, `sha256` kitöltése, licenc, próba `flatpak-builder`-rel |
| `nix/package.nix` | nixpkgs | a flake már működik; nixpkgs PR-hez publikus forrás és `fetchFromGitHub` kell |

Flatpakban az adatmappa `~/.var/app/hu.kriptoado.app/data/hu.kriptoado.app/`, és a fájlválasztó miatt a saját mappához hozzáférést kér (`--filesystem=home`).

A kimenetek a `telepito/windows` és `telepito/linux` mappába kerülnek (`kriptomerleg_<verzió>_…` néven).

Linux függőségek (Debian/Ubuntu):

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

## NixOS

A repóban van egy Nix flake (`flake.nix`, `nix/package.nix`), ebből a NixOS natívan fordítja az appot.

Futtatás telepítés nélkül (a repó privát, ezért SSH-n keresztül):

```bash
nix run git+ssh://git@github.com/adamgyongyosi/kriptomerleg   # privát repóból
nix run github:adamgyongyosi/kriptomerleg                   # ha a repó publikus
```

Rendszerszintű telepítés flake-alapú NixOS konfigurációban:

```nix
# flake.nix
inputs.kriptomerleg.url = "git+ssh://git@github.com/adamgyongyosi/kriptomerleg";

# configuration.nix (a modulnak átadott inputs-szal)
environment.systemPackages = [ inputs.kriptomerleg.packages.${pkgs.stdenv.hostPlatform.system}.default ];
```

Fejlesztői környezet: `nix develop`, utána `npm install && npm run tauri dev`. A fájlválasztó megnyitása előtt az app Linuxon ellenőrzi, hogy a séma elérhető-e; ha nem, összeomlás helyett hibaüzenetet ad. A megerősítő és beviteli ablakok az appon belül jelennek meg, nem a rendszer (GTK) üzenetablakában. Az appot mindig a `nix develop` shellből indítsd: ez állítja be az `XDG_DATA_DIRS`-t a GTK fájlválasztó sémáira (`org.gtk.Settings.FileChooser`). Nélküle a CSV-export és az import fájlválasztója `Settings schema 'org.gtk.Settings.FileChooser' is not installed` hibával összeomlik ([#1](https://github.com/adamgyongyosi/kriptomerleg/issues/1)). A `nix run` / `nix build` csomagban ezt a `wrapGAppsHook3` intézi (`gtk3` és `gsettings-desktop-schemas` a függőségek között); a CI mindkét esetet ellenőrzi.

Alternatíva: az AppImage NixOS-en `appimage-run`-nal indítható (`programs.appimage.enable = true;`).

## Biztonság

- **Tartalombiztonsági szabály (CSP):** a felület csak a saját fájljait töltheti be, hálózatra csak a Tauri IPC-n át „beszélhet” (`connect-src ipc: http://ipc.localhost`), objektum és űrlapküldés tiltva (`tauri.conf.json` → `app.security.csp`). Az MNB-, Kraken- és frissítéslekérés a Rust oldalon fut, nem a webnézetben. Fejlesztői módban (`devCsp: null`) a Vite HMR miatt nincs CSP.
- **Jogosultságok** (`src-tauri/capabilities/default.json`): a fájlkezelőben csak az app saját adatmappája nyitható meg (`$APPDATA/**`, a mentések mappájához); a frissítő csak a `tauri.conf.json`-ban rögzített nyilvános kulccsal aláírt csomagot telepít.
- **Titkok:** a Kraken API kulcsot és titkot az app nem menti, és nem naplózza. A mentésjelszót sem tárolja (Argon2id + ChaCha20-Poly1305). Az SQL lekérdezések kötött paraméterekkel futnak.
- **Ki módosíthat:** a `main` ágat ruleset védi: force push és törlés tiltva, változás csak pull requestből, amelyet a `.github/CODEOWNERS` szerint a tulajdonosnak (@adamgyongyosi) kell jóváhagynia; a repó adminja kivétel. Kívülállók csak forkolhatnak és PR-t nyithatnak; a forkból jövő PR-ek nem kapnak titkot.
- **Aláírókulcs környezetben:** a `TAURI_SIGNING_PRIVATE_KEY` a `release` környezet titka (Settings → Environments → release), amelyet csak a `main` ág használhat; a build job csak a `main`-re pusholt változásnál kéri ezt a környezetet. Egy `v*` tagből induló kiadás ezért aláíratlan (az automatikus frissítő nem ajánlja fel); a rendes kiadások a `main`-ből jönnek.
- **Tagek és kiadások:** a `v*` tageket tag-ruleset védi (módosítás, törlés, force push tiltva; az admin kivétel), és a release-ek változtathatatlanok (*Release immutability*): a kiadott fájlok és a tag utólag nem cserélhetők. A CI ezért előbb piszkozatot készít az összes fájllal, és csak utána teszi közzé (`gh release edit --draft=false --latest`).
- **Collaboratorok:** személyes repóban a collaboratornak mindig írási joga van, így új release-t vagy ágat létrehozhat. Akinek nem kell írási jog, azt érdemes eltávolítani (Settings → Collaborators); finomabb szerepkörökhöz (Read, Triage) szervezet kell.
- **Frissítő kulcs:** a privát aláírókulcs csak *Secret*-ként kerülhet a GitHubra (Settings → Secrets and variables → Actions → **Secrets** → New repository secret), soha nem *Variable*-ként, mert a változók értéke mindenkinek olvasható, aki hozzáfér a repóhoz. 2026-10-31-én a kulcspárt lecseréltük, mert a régi privát kulcs változóként került fel.
- **Függőségek:** a CI `audit` lépése minden pushnál lefuttatja a `cargo audit`-ot (RustSec) és az `npm audit --omit=dev`-et. Helyben: `cargo install cargo-audit`, majd `cd src-tauri && cargo audit`. Tudatos kivétel indoklással: `src-tauri/.cargo/audit.toml` (`RUSTSEC-2026-0187`, lopdf: csak PDF *beolvasásakor* érint, az app PDF-et csak ír). A fennmaradó figyelmeztetések (karbantartatlan / unsound crate-ek) a Tauri GTK- és D-Bus-függőségeiből jönnek.
- **Biztonsági audit (2026-10-31):** a 0.3.0 változásainak átnézése (fájlútvonalas parancsok, profilazonosítók, mentés-visszaállítás, titkosítás, Kraken API, import, frissítő, CI) nem talált kihasználható hibát. Javítva: a Vitest fejlesztői függőség kritikus sérülékenysége (5-ös verzióra emelve), a CSP bekapcsolva, a mappamegnyitási jog szűkítve.

## Weboldal

A landing oldal a `site/` mappában van: egyetlen statikus `index.html` (külső függőség nélkül), a logó és az ikon. Sötét/világos módot és mobilnézetet is kezel, a letöltőgombok a legfrissebb release fájljaira mutatnak, és kiemeli a látogató rendszeréhez illőt.

- Helyi előnézet: `npx vite` (vagy `npm run dev`), majd <http://localhost:1420/site/index.html>
- Közzététel: a `.github/workflows/pages.yml` minden `site/` változásnál feltölti a GitHub Pages-re.
- **Egyszeri beállítás:** Settings → Pages → Source: *GitHub Actions*. Ingyenes GitHub-fiókkal a Pages (és a mindenki által letölthető release) csak **publikus** repóból működik.
- A repó neve egy helyen szerepel a lap alján lévő szkriptben (`REPO`), átnevezésnél csak ezt kell módosítani.
- A funkciólista (9 kártya), az adatvédelmi sáv, az összevetés és a GYIK az app funkcióit követi; új funkciónál ezeket is frissítsd.

### Keresőoptimalizálás (SEO, GEO, AEO)

- **Cím és leírás:** a `<title>` és a meta description a fő keresett kifejezésekkel („kriptó adó kalkulátor”, „SZJA”, „NAV bevallás”); a H1 felső sora ugyanezt tartalmazza, a szlogen megmarad.
- **Kanonikus URL és megosztás:** `canonical`, Open Graph (`og:url`, `og:locale` = `hu_HU`, 1200×630-as `og-image.png`) és Twitter-kártya. A megosztási kép forrása `brand/og-card.html`; újragenerálás Edge-dzsel a fájl elején leírt paranccsal.
- **Strukturált adat (JSON-LD):** `WebSite`, `SoftwareApplication` (ingyenes, Windows/Linux, `FinanceApplication`, letöltési link) és `FAQPage`. A `FAQPage` szövege pontosan a látható GYIK; ha a GYIK változik, a JSON-LD-t is frissíteni kell (a Google eltérés esetén figyelmen kívül hagyja).
- **AI-válaszmotorok (GEO/AEO):** a GYIK a valódi keresési kérdésekre (mennyi a kriptóadó, adóköteles-e a kripto–kripto csere, mikor kell bevallani, adókiegyenlítés, kisértékű ügylet) ad rövid, tényszerű, törvényhivatkozásos választ. A `site/llms.txt` gépi olvasásra szánt összefoglaló a fő tényekkel és linkekkel. Az oldalon szerepel a frissítés dátuma (`<time>`), a JSON-LD-ben `dateModified`.
- **Sitemap:** `site/sitemap.xml`. Projektoldalon (`/kriptomerleg/`) saját `robots.txt` nem lehet (az csak a domain gyökerében működik), ezért a sitemapet kézzel kell beküldeni:
  1. [Google Search Console](https://search.google.com/search-console) → *Tulajdon hozzáadása* → *URL-előtag*: `https://adamgyongyosi.github.io/kriptomerleg/` → ellenőrzés *HTML-címkével*: a kapott `<meta name="google-site-verification" …>` sort tedd a `site/index.html` `<head>`-jébe, pushold, majd *Ellenőrzés*.
  2. *Webhelytérképek* → `sitemap.xml` → *Beküldés*. Az *URL-vizsgálat* → *Indexelés kérése* gyorsítja az első indexelést.
  3. [Bing Webmaster Tools](https://www.bing.com/webmasters) → *Importálás a Google Search Console-ból* (a Bing adja a ChatGPT keresés találatainak egy részét is).
- Ellenőrzés: [Rich Results Test](https://search.google.com/test/rich-results) és [Schema Markup Validator](https://validator.schema.org/) az oldal URL-jével.

Arculat (logó, színek, hangnem): [`brand/README.md`](brand/README.md).

## Felépítés

- `src-tauri/src/tax.rs` – adószámítás, adókiegyenlítés, kisértékű ügylet, minimálbér-tábla (unit tesztekkel)
- `src-tauri/src/mnb.rs` – MNB SOAP árfolyam-lekérés
- `src-tauri/src/excel.rs` – régi Excel import
- `src-tauri/src/import/` – tőzsdei import: közös CSV/Excel olvasó, szám- és dátumértelmezés (`mod.rs`), formátumonként `binance.rs`, `revolut.rs`, `kraken.rs`, `bybit.rs`, `bank.rs` (banki kivonat), `kraken_api.rs` (Kraken API, aláírással) – mintafájlos tesztekkel
- `src-tauri/src/lib.rs` – SQLite tárolás, Tauri parancsok, CSV export, önellenőrzés-segéd
- `src-tauri/src/data.rs` – Excel és PDF export, beállítások, mentés/visszaállítás, profilok, fájlválasztó-ellenőrzés
- `src-tauri/src/backup.rs` – mentésfájl írása/olvasása, titkosítás, heti automatikus mentés
- `src-tauri/src/pdf.rs` – éves nyilvántartás PDF-ben
- `src/` – React felület: `App.tsx` (oldalak), `EntryForm.tsx`, `Filing.tsx` (bevallási segéd), `DataPage.tsx` (adatok és mentés), `ImportDialog.tsx`, `Dialogs.tsx` (appon belüli párbeszédablakok), `updates.ts` (frissítő); tesztek: `*.test.ts(x)`
- `packaging/` – AUR és Flatpak csomagleírás
- `site/` – landing oldal (`index.html`, `og-image.png`, `sitemap.xml`, `llms.txt`); `brand/og-card.html` – a megosztási kép forrása
- `brand/` – logó és arculati leírás
- `site/` – landing oldal (GitHub Pages)
- `src/` – React felület (`App.tsx`, `EntryForm.tsx`, `ImportDialog.tsx`, `Filing.tsx`, `Chart.tsx`)
