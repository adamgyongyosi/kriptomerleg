import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { inDesktopApp } from "./api";

export const RELEASES_URL = "https://github.com/adamgyongyosi/kripto-ado/releases/latest";

/**
 * Új verzió keresése a GitHub Release `latest.json` fájlja alapján. Hálózati hiba vagy hiányzó
 * fájl esetén (pl. aláíratlan build) `null`, ha `quiet`; különben a hibát továbbadja.
 */
export async function findUpdate(quiet = true): Promise<Update | null> {
  if (!inDesktopApp()) return null;
  try {
    return await check({ timeout: 15000 });
  } catch (e) {
    if (quiet) return null;
    throw new Error(`Nem sikerült frissítést keresni: ${e}`);
  }
}

/** Letöltés, telepítés és újraindítás. Linuxon csak az AppImage frissíthető így. */
export async function installUpdate(update: Update, onProgress?: (percent: number | null) => void): Promise<void> {
  let total = 0;
  let done = 0;
  try {
    await update.downloadAndInstall((ev) => {
      if (ev.event === "Started") total = ev.data.contentLength ?? 0;
      if (ev.event === "Progress") {
        done += ev.data.chunkLength;
        onProgress?.(total ? Math.round((done / total) * 100) : null);
      }
    });
  } catch (e) {
    throw new Error(
      `A frissítés nem telepíthető automatikusan (${e}). Töltsd le kézzel a kiadások oldaláról (.deb csomagnál ez a megoldás).`,
    );
  }
  await relaunch();
}
