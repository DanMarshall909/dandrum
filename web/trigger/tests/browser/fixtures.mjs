import {test as base,expect} from '@playwright/test';
import {readFile,writeFile} from 'node:fs/promises';
import {servedOwner} from '../../scripts/browser-coverage.mjs';

const inventory=process.env.TRIGGER_BROWSER_COVERAGE?
  JSON.parse(await readFile(process.env.TRIGGER_BROWSER_COVERAGE,'utf8')):null;
export const test=base.extend({
  _sourceCoverage:[async({page},use,testInfo)=>{
    if(!inventory){await use();return;}
    await page.coverage.startJSCoverage({resetOnNavigation:false});
    try{await use();}finally{
      const entries=(await page.coverage.stopJSCoverage()).filter(entry=>
        /^https?:/.test(entry.url)&&inventory.browserOwners.includes(servedOwner(entry.url,inventory.root)));
      const path=testInfo.outputPath('browser-coverage.json');
      await writeFile(path,JSON.stringify({runId:inventory.runId,testId:testInfo.testId,status:testInfo.status,
        browserVersion:page.context().browser().version(),entries}));
      await testInfo.attach('browser-coverage',{path,contentType:'application/json'});
    }
  },{auto:true}],
});
export {expect};
