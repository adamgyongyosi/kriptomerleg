import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Felület-tesztek: jsdom böngészőkörnyezet, a Tauri hívásokat a @tauri-apps/api/mocks helyettesíti.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
