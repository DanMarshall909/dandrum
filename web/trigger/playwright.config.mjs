import {defineConfig} from '@playwright/test';
export default defineConfig({testDir:'tests/browser',fullyParallel:false,workers:1,
  use:{baseURL:'http://127.0.0.1:8322',viewport:{width:1800,height:1200},launchOptions:{executablePath:process.env.TRIGGER_CHROME??'/usr/bin/google-chrome'}},
  webServer:{command:'npm run dev -- --port 8322 --strictPort',url:'http://127.0.0.1:8322',reuseExistingServer:true}});
