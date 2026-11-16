import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (
            /node_modules\/(motion|framer-motion|motion-dom|motion-utils)\//.test(
              id,
            )
          )
            return "interaction-vendor";
        },
      },
    },
  },
  plugins: [react()],
  resolve: { alias: { "react-native": "react-native-web" } },
});
