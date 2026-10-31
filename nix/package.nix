{
  lib,
  rustPlatform,
  cargo-tauri,
  nodejs,
  importNpmLock,
  pkg-config,
  wrapGAppsHook3,
  glib-networking,
  gsettings-desktop-schemas,
  gtk3,
  librsvg,
  openssl,
  webkitgtk_4_1,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "kriptomerleg";
  version = "0.3.0";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../index.html
      ../package.json
      ../package-lock.json
      ../tsconfig.json
      ../tsconfig.node.json
      ../vite.config.ts
      ../examples
      ../src
      ../src-tauri
    ];
  };

  cargoRoot = "src-tauri";
  buildAndTestSubdir = finalAttrs.cargoRoot;
  cargoLock.lockFile = ../src-tauri/Cargo.lock;

  # Hash nélküli npm függőségek a package-lock.json alapján.
  npmDeps = importNpmLock { npmRoot = ../.; };

  nativeBuildInputs = [
    cargo-tauri.hook
    nodejs
    importNpmLock.npmConfigHook
    pkg-config
    wrapGAppsHook3
  ];

  # A gtk3 és a gsettings-desktop-schemas GSettings-sémáit a wrapGAppsHook3 teszi
  # az XDG_DATA_DIRS-be; nélkülük a fájlválasztó (CSV-export, import) összeomlik.
  buildInputs = [
    glib-networking
    gsettings-desktop-schemas
    gtk3
    librsvg
    openssl
    webkitgtk_4_1
  ];

  meta = {
    description = "Kriptomérleg – magyar kriptó-SZJA kalkulátor, helyben futó asztali app";
    homepage = "https://github.com/adamgyongyosi/kripto-ado";
    mainProgram = "kriptomerleg";
    platforms = lib.platforms.linux;
  };
})
