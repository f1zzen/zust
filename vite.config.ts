// @ts-ignore
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
// @ts-ignore
import dns from "node:dns";

// @ts-ignore
const host = process.env.TAURI_DEV_HOST;

dns.setDefaultResultOrder("verbatim");

export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
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
  }
}));
