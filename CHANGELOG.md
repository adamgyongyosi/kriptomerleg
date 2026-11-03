# Változásnapló

A Kriptomérleg minden fontos változása ebben a fájlban szerepel.

A formátum a [Keep a Changelog](https://keepachangelog.com/hu/1.1.0/) ajánlását követi. A verziószám `<fő>.<al>.<commitszám>` alakú (lásd [docs/fejlesztes.md](docs/fejlesztes.md#verzió-és-kiadás)).

## [Unreleased]

## [0.3.28] – 2026-11-03

### Változott

- A README rövid áttekintő lett; a részletes leírások a `docs/` mappába kerültek (adószámítás, import, adatok, fejlesztés, biztonság, weboldal).
- A verziószám harmadik tagja mostantól a `main` ág commitjainak száma, nem a CI futás sorszáma. Így a verzió előre tudható, és bekerülhet a változásnaplóba; ezért a 0.3.21 után a 0.3.28 következik.
- A kiadás leírása és a frissítő értesítése a változásnapló adott verziójú szakaszát mutatja, nem csak az utolsó commit üzenetét.

### Hozzáadva

- Változásnapló (`CHANGELOG.md`) a teljes eddigi történettel, és `scripts/changelog.mjs` a kiadási szakasz elnevezéséhez és a kiadási jegyzethez.

## [0.3.21] – 2026-11-03

### Javítva

- Weboldal: a képnagyító lapozó- és bezárógombjainak ikonja pontosan középen áll, és a gomb köre alul nem vágódik le.

## [0.3.20] – 2026-11-02

### Hozzáadva

- Weboldal: modern animációk – mozgó színfoltok és színátmenetes cím, a hero kép 3D-s beúszása és egérkövető dőlése, források futószalagon, görgetésre beúszó blokkok, görgetéshez kötött ráközelítés és haladásjelző, kurzort követő fény a kártyákon. Csökkentett mozgás beállításnál mind kikapcsol.

## [0.3.19] – 2026-11-02

### Hozzáadva

- Weboldal: valódi képernyőképek a programból (sötét és világos változat, 1×/2× WebP), „Bemutató” rész öt kép–szöveg blokkal, képnagyító lapozással.
- A képernyőképek bemutató adatokkal, a valódi adószámítással generálhatók újra (`dump_demo` teszt, `brand/app-screens.*`).

### Változott

- Weboldal: átdolgozott felépítés, mobilon görgethető menü, lenyíló teljes funkciólista, záró felhívás; a sitemap a képeket is felsorolja.

### Javítva

- Adótervező: 100 Ft alatti, csak kerekítésből adódó eltérésnél „ugyanannyi” az eredmény, nem „1 Ft-tal kevesebb”.
- A Crypto.com forrás neve helyesen jelenik meg a megjegyzésekben, az ellenőrzésben és a PDF-ben („Cryptocom” helyett).

## [0.3.18] – 2026-11-02

### Hozzáadva

- Ügyletszintű nyomkövetés: az importált napi tétel mögött az eredeti sorok és a forrás (fájlnév + SHA-256, vagy API + lekérési idő); megtekinthető a Tételek oldalon, az Excel-exportban „Eredeti sorok” lap, a PDF-ben melléklet.
- Binance és Bybit lekérés csak olvasó API kulccsal, közös API-ablak a Krakennel.
- Crypto.com App import; a Visa kártya kriptós feltöltése „Fizetés kriptóval” tétel.
- Új tételtípus: kriptóban kapott jövedelem (kiadásnak számít).
- Ellenőrzés: vétel nélküli bevétel, banki és tőzsdei átfedés, dupla tétel, hézagos és elavult import.
- Adótervező: kivétel idén vagy jövőre, a kisértékű ügylet keretével.

### Változott

- Binance: a fiat egyenlegből fizetett kártyás költés nem számít vételnek; a kriptós költést napra bontva jelzi kézi rögzítéshez.

## [0.3.17] – 2026-11-02

### Változott

- Gyorsabb import: a mentés egyetlen adatbázis-tranzakcióban történik.
- A hétvégi és ünnepnapi MNB árfolyamot a futás idejére megjegyzi; az Áttekintés euróárfolyamát naponta egyszer kéri le.

### Javítva

- Gyors egymás utáni üzeneteknél a korábbi időzítő nem tünteti el az újat.

## [0.3.16] – 2026-10-31

### Hozzáadva

- Fejlesztői útmutató AI kódolóasszisztenshez (`CLAUDE.md`).

## [0.3.15] – 2026-10-31

### Hozzáadva

- Weboldal: kipróbálható mockup (befizetés, kivétel, fizetés kriptóval, € $ £ Ft), „Minden funkció részletesen” lista.

## [0.3.14] – 2026-10-31

### Hozzáadva

- Weboldal: keresőoptimalizálás – kulcsszavas cím és leírás, Open Graph megosztási kép, JSON-LD (WebSite, SoftwareApplication, FAQPage), új GYIK, `sitemap.xml`, `llms.txt`.

### Javítva

- A régi aláírókulccsal készült példány érthető hibaüzenetet ad a frissítéskor.

## [0.3.13] – 2026-10-31

### Változott

- Minden kiadásnak valódi, növekvő verziószáma van (pl. 0.3.13), ez kerül az appba is, így a frissítő minden új kiadást felajánl.
- A repó neve `kripto-ado` helyett `kriptomerleg`.
- A kiadás előbb piszkozatként készül el az összes fájllal, és csak utána lesz közzétéve.

### Biztonság

- Új frissítő-aláírókulcs; a régi privát kulcs véletlenül nem titkos változóként került a GitHubra. A korábbi példányokat egyszer kézzel kell frissíteni.
- Az aláírókulcs a `release` környezet titka, csak a `main` ágról induló build éri el; CODEOWNERS és ág-, tagvédelem.

## [0.3.0] – 2026-10-31

### Hozzáadva

- USD, GBP és CHF tételek MNB árfolyammal.
- Új tételtípus: fizetés kriptóval (bevételnek számít).
- Import előnézet: a más forrásból már meglévő napi tétel „lehet dupla” jelölést kap.
- Áttekintés: az adómentesen kivehető összeg euróban is.
- Adatok és mentés oldal: mentés fájlba (jelszóval titkosítva is), visszaállítás, heti automatikus mentés, Excel (.xlsx) export.
- Több profil, saját adatbázissal.
- Határidő-emlékeztető rendszerértesítéssel.
- Kisértékű ügylet (67/C. § (2)) kezelése minimálbér-táblával.
- Önellenőrzés-segéd a bevallatlan veszteséges évekre.
- Éves nyilvántartás PDF-ben; a bevallási sorok CSV-be menthetők.
- Banki kivonat import (Revolut, Wise, magyar bankok) és Kraken lekérés API kulccsal.
- Automatikus frissítés aláírt csomagokkal.
- Felület-tesztek (Vitest), AUR és Flatpak csomagleírás.

### Változott

- A megerősítő és beviteli ablakok az appon belül jelennek meg.

### Javítva

- Linuxon a fájlválasztó hiányzó séma esetén hibaüzenetet ad, nem omlik össze.
- A színes gombok szövege hoverkor is olvasható.

### Biztonság

- Tartalombiztonsági szabály (CSP), szűkített fájlkezelő-jogosultság, `cargo audit` és `npm audit` a CI-ben; a Vitest sérülékeny függősége javítva.

## [0.2.0-build.7] – 2026-10-31

### Javítva

- NixOS: a CSV-export és az import fájlválasztója nem omlik össze (GTK sémák a csomagban és a `nix develop` shellben). ([#1](https://github.com/adamgyongyosi/kriptomerleg/issues/1))

## [0.2.0-build.6] – 2026-10-29

### Hozzáadva

- Évenként jelölhető, melyik év veszteségét vallottad be; az Évek oldalon „Bevallva” oszlop.
- Az összeg mező ezres tagolást kap gépelés közben.

### Javítva

- A 2022 előtti tételek a 2022-es eredménybe számítanak (NAV-szabály).
- Korábbi év vesztesége csak akkor vonható le, ha bevallották; a lejárt határidejű év alapból bevallatlan.

## [0.2.0-build.5] – 2026-10-26

### Hozzáadva

- Tőzsdei import: Binance, Revolut, Kraken és Bybit export, naponta összevonva, díjakkal, MNB árfolyamon; újraimportálás nem duplikál.
- Importálható mintafájlok (`examples/`).
- Automatikus GitHub Release minden `main`-re pusholt változásból.
- Kriptomérleg név, logó, ikonok és weboldal.

### Változott

- Deviza választó helyett € / Ft kapcsoló, alapból euró.

## [0.1.0] – 2026-10-26

### Hozzáadva

- Első kiadás: EUR/HUF befizetések és kivételek MNB árfolyammal, 15% SZJA és adókiegyenlítés, bevallási segéd, régi Excel import, CSV export, grafikon.
- Windows és Linux build, NixOS flake.

[Unreleased]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.3.28...HEAD
[0.3.28]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.3.21...v0.3.28
[0.3.21]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.3.20...v0.3.21
[0.3.20]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.3.19...v0.3.20
[0.3.19]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.3.18...v0.3.19
[0.3.18]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.3.17...v0.3.18
[0.3.17]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.3.16...v0.3.17
[0.3.16]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.3.15...v0.3.16
[0.3.15]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.3.14...v0.3.15
[0.3.14]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.3.13...v0.3.14
[0.3.13]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.2.0-build.7...v0.3.13
[0.2.0-build.7]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.2.0-build.6...v0.2.0-build.7
[0.2.0-build.6]: https://github.com/adamgyongyosi/kriptomerleg/compare/v0.2.0-build.5...v0.2.0-build.6
[0.2.0-build.5]: https://github.com/adamgyongyosi/kriptomerleg/releases/tag/v0.2.0-build.5
