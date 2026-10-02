import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fontLicenseBanner } from '../shared/font-licenses.mjs';

export default defineConfig({
  plugins: [react()],
  base: '/',
  build: {
    assetsInlineLimit: Infinity,
    assetsDir: '',
    rollupOptions: {
      output: {
        banner: fontLicenseBanner,
        entryFileNames: 'app.js',
        assetFileNames: 'app[extname]',
      },
    },
  },
});
