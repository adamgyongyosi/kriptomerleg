# Kripto adó

Asztali alkalmazás (Windows, Linux) a kriptóba befizetett és onnan kivett összegek nyilvántartására és a magyar kriptó-SZJA kiszámítására.

- EUR tételeknél az MNB hivatalos árfolyamát automatikusan lekéri a teljesítés napjára (hétvégén/ünnepnap az utolsó közzétettet), forintos tétel is rögzíthető.
- Éves eredmény = kivételek − befizetések; nyereség után 15% SZJA.
- Adókiegyenlítés: a veszteség 15%-a levonható, legfeljebb a tárgyévi és az előző két évi kriptoadó erejéig; a 2025-ös bevallástól a veszteség nem évül el (2025. évi LXXXIII. tv.).
- Bevallási segéd: mit kell a 25SZJA 164. sorába és a 06-os lap 202–209. sorába írni (NAV kitöltési útmutató alapján).
- Régi Excel import (évszám nevű lapok, A: dátum, B: forint), CSV export.
- Adatok helyben, SQLite-ban.

> Tájékoztató számítás, nem adótanácsadás.

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
- GitHub Actions: minden pushnál Windows (NSIS) és Linux (.deb, AppImage) build, letölthető artifactként, valamint egy `nix build` ellenőrzés a NixOS-csomaghoz.

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
- `src-tauri/src/lib.rs` – SQLite tárolás, Tauri parancsok, CSV export
- `src/` – React felület (`App.tsx`, `EntryForm.tsx`, `Filing.tsx`, `Chart.tsx`)
