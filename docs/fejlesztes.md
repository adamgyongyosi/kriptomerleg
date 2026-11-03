# Fejlesztés és build

[← vissza a README-hez](../README.md)

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
- A helyi szkriptek kimenetei a `telepito/windows` és `telepito/linux` mappába kerülnek (`kriptomerleg_<verzió>_…` néven).

Linux függőségek (Debian/Ubuntu):

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

## Verzió és kiadás

- **Verziószám:** `<fő>.<al>.<commitszám>`. A fő- és alverzió a `package.json`-ból jön, a harmadik tag a `main` ág commitjainak száma (`git rev-list --count HEAD`). Így minden kiadás verziója előre tudható, és mindig nő, ezért az automatikus frissítő minden új kiadást felajánl. Egy `v*` tag (pl. `v1.0.0`) pontosan azt a verziót adja ki.
- **Fő- vagy alverzió emelése:** `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `nix/package.nix` és a `packaging/` fájljai együtt.
- **Változásnapló:** a [`CHANGELOG.md`](../CHANGELOG.md) a [Keep a Changelog](https://keepachangelog.com/hu/1.1.0/) formátumát követi (Hozzáadva / Változott / Elavult / Eltávolítva / Javítva / Biztonság). Munka közben a bejegyzések a `## [Unreleased]` alá kerülnek. A `main`-re pusholás előtti utolsó commitban ezt nevezd át a kiadás verziójára és dátumára, és frissítsd a lap alji összehasonlító linkeket:

  ```bash
  node scripts/changelog.mjs release
  ```

  A szkript a következő commit számából kiszámolja a verziót (`<fő>.<al>.<rev-list --count HEAD + 1>`), átnevezi az `[Unreleased]` szakaszt, és új, üres `[Unreleased]`-t nyit. Ha a push több commitot tartalmaz, az utolsóban futtasd.
- **Kiadási jegyzet:** a CI a release leírásába és a frissítő `latest.json`-jába a `CHANGELOG.md` adott verziójú szakaszát teszi; ha nincs ilyen, az `[Unreleased]` szakaszt, ha az is üres, a commit üzenetét.

## Automatikus frissítés (updater)

- Az app a `https://github.com/adamgyongyosi/kriptomerleg/releases/latest/download/latest.json` fájlból tudja meg a legújabb verziót; a csomagokat a `tauri.conf.json`-ban lévő nyilvános kulccsal ellenőrzi.
- Az aláíráshoz a CI-nek két titok kell (*Settings → Secrets and variables → Actions*): `TAURI_SIGNING_PRIVATE_KEY` (a privát kulcsfájl tartalma) és `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (üres, ha a kulcsnak nincs jelszava). Ha nincsenek beállítva, a build ugyanúgy elkészül, csak `latest.json` és `.sig` fájlok nélkül.
- Új kulcspár: `npx tauri signer generate -w ~/.tauri/kriptomerleg-updater.key`, majd a `.pub` tartalma a `tauri.conf.json` `plugins.updater.pubkey` mezőjébe. A privát kulcs ne kerüljön a repóba.
- A frissítés csak publikus repóból működik (privát repó kiadásai bejelentkezés nélkül nem tölthetők le); ugyanez igaz a weboldal letöltőgombjaira.
- A 2026. október 31. előtt készült példányok (a 0.3.13 előtti kiadások és a korábbi helyi buildek) még a lecserélt kulcs nyilvános részét tartalmazzák, ezért „The signature was created with a different key” hibát adnak. Ezeket egyszer kézzel kell frissíteni a legújabb kiadásra; onnantól az automatikus frissítés működik. Az app ilyenkor ezt írja ki.

## Linux csomagtárak

A `packaging/` mappában előkészített, még be nem küldött csomagleírások:

| Fájl | Hová | Teendő beküldés előtt |
|---|---|---|
| `packaging/aur/PKGBUILD` | AUR (`kriptomerleg-bin`, a release .deb fájljából) | publikus repó, `v<verzió>` tag, licenc, `updpkgsums` |
| `packaging/flatpak/hu.kriptoado.app.yml` + `.metainfo.xml` | Flathub | publikus repó, `sha256` kitöltése, licenc, próba `flatpak-builder`-rel |
| `nix/package.nix` | nixpkgs | a flake már működik; nixpkgs PR-hez publikus forrás és `fetchFromGitHub` kell |

Flatpakban az adatmappa `~/.var/app/hu.kriptoado.app/data/hu.kriptoado.app/`, és a fájlválasztó miatt a saját mappához hozzáférést kér (`--filesystem=home`).

## NixOS fejlesztői környezet

`nix develop`, utána `npm install && npm run tauri dev`. A fájlválasztó megnyitása előtt az app Linuxon ellenőrzi, hogy a séma elérhető-e; ha nem, összeomlás helyett hibaüzenetet ad. A megerősítő és beviteli ablakok az appon belül jelennek meg, nem a rendszer (GTK) üzenetablakában. Az appot mindig a `nix develop` shellből indítsd: ez állítja be az `XDG_DATA_DIRS`-t a GTK fájlválasztó sémáira (`org.gtk.Settings.FileChooser`). Nélküle a CSV-export és az import fájlválasztója `Settings schema 'org.gtk.Settings.FileChooser' is not installed` hibával összeomlik ([#1](https://github.com/adamgyongyosi/kriptomerleg/issues/1)). A `nix run` / `nix build` csomagban ezt a `wrapGAppsHook3` intézi (`gtk3` és `gsettings-desktop-schemas` a függőségek között); a CI mindkét esetet ellenőrzi.

## Felépítés

- `src-tauri/src/tax.rs` – adószámítás, adókiegyenlítés, kisértékű ügylet, minimálbér-tábla (unit tesztekkel)
- `src-tauri/src/mnb.rs` – MNB SOAP árfolyam-lekérés
- `src-tauri/src/excel.rs` – régi Excel import
- `src-tauri/src/import/` – tőzsdei import: közös CSV/Excel olvasó, szám- és dátumértelmezés (`mod.rs`), formátumonként `binance.rs`, `revolut.rs`, `kraken.rs`, `bybit.rs`, `cryptocom.rs`, `bank.rs` (banki kivonat), `kraken_api.rs`, `binance_api.rs`, `bybit_api.rs` (API-s lekérés, aláírással) – mintafájlos tesztekkel
- `src-tauri/src/lib.rs` – SQLite tárolás, Tauri parancsok, CSV export, önellenőrzés-segéd
- `src-tauri/src/checks.rs` – ellenőrzés (hiányzó, dupla, hézagos tételek) és adótervező (kivétel idén vagy jövőre, kisértékű keret)
- `src-tauri/src/data.rs` – Excel és PDF export, beállítások, mentés/visszaállítás, profilok, fájlválasztó-ellenőrzés
- `src-tauri/src/backup.rs` – mentésfájl írása/olvasása, titkosítás, heti automatikus mentés
- `src-tauri/src/pdf.rs` – éves nyilvántartás PDF-ben
- `src/` – React felület: `App.tsx` (oldalak), `EntryForm.tsx`, `Filing.tsx` (bevallási segéd), `DataPage.tsx` (adatok és mentés), `ImportDialog.tsx`, `Checks.tsx` (ellenőrzés, eredeti sorok, adótervező), `Dialogs.tsx` (appon belüli párbeszédablakok), `updates.ts` (frissítő); tesztek: `*.test.ts(x)`
- `packaging/` – AUR és Flatpak csomagleírás
- `site/` – landing oldal (`index.html`, `screens/` képernyőképek, `og-image.png`, `sitemap.xml`, `llms.txt`); `brand/og-card.html` – a megosztási kép forrása; `brand/app-screens.*` – a képernyőképek bemutató felülete és rögzítő szkriptje
- `brand/` – logó és arculati leírás
- `docs/` – részletes leírások (adószámítás, import, adatok, fejlesztés, biztonság, weboldal); `CHANGELOG.md` – változásnapló
- `CLAUDE.md` – tömör fejlesztői útmutató AI kódolóasszisztensnek (Claude Code): parancsok, architektúra, konvenciók; nagyobb szerkezeti változásnál ezt is igazítsd
