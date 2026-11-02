import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Month and year of this build, shown next to the version.
  define: { __BUILD_DATE__: JSON.stringify(new Date().toLocaleDateString("en-US", { month: "long", year: "numeric" })) },
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: "es2021", outDir: "dist" },
});
