import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
        protocol: "ws",
        host: host,
        port: 1421,
      }
      : {
        protocol: "ws",
        host: "127.0.0.1",
        port: 1421,
      },
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
