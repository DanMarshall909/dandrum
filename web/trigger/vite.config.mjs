import {defineConfig} from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({plugins:[react()],server:{host:'127.0.0.1'},build:{rollupOptions:{input:{shell:'index.html',catalog:'catalog.html'},output:{manualChunks(id){
  if(id.includes('/node_modules/'))return 'react-runtime';
  if(id.includes('/vendor/design-system/'))return 'design-system';
}}}}});
