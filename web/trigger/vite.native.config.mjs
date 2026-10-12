import {defineConfig} from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins:[react()],
  build:{
    outDir:'dist-native',
    rollupOptions:{
      input:'index.html',
      output:{
        entryFileNames:'app.js',
        inlineDynamicImports:true,
        assetFileNames:asset=>asset.names?.some(name=>name.endsWith('.css'))?'app.css':'assets/[name][extname]',
      },
    },
  },
});
