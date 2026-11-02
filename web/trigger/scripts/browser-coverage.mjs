import {createHash, randomUUID} from 'node:crypto';
import {readFile, writeFile, mkdir, readdir} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {dirname, resolve, join, isAbsolute} from 'node:path';
import {fileURLToPath} from 'node:url';
import convertSourceMap from 'convert-source-map';
import v8ToIstanbul from 'v8-to-istanbul';
import {TraceMap, decodedMappings, originalPositionFor} from '@jridgewell/trace-mapping';

const hash=source=>createHash('sha256').update(source).digest('hex');
const root=fileURLToPath(new URL('../',import.meta.url));
const selectedOwners=inventory=>inventory.browserOwners??Object.keys(inventory.files);
export function servedOwner(url,base){
  const path=decodeURIComponent(new URL(url).pathname);
  return path.startsWith('/@fs/')?path.slice(4):resolve(base,'.'+path);
}

export async function convertCapture(inventory,capture){
  if(capture.runId!==inventory.runId)throw Error('coverage foreign run');
  const lines={},istanbul=[],generated=[];
  for(const entry of capture.entries){
    const owner=servedOwner(entry.url,inventory.root), original=inventory.files[owner];
    if(!original||!selectedOwners(inventory).includes(owner))continue;
    if(!Array.isArray(entry.functions)||entry.functions.some(fn=>!Array.isArray(fn.ranges)||!fn.ranges.length))throw Error('coverage missing whole-program evidence: '+owner);
    for(const fn of entry.functions)for(const range of fn.ranges){
      if(!Number.isInteger(range.startOffset)||!Number.isInteger(range.endOffset)||range.startOffset<0||range.endOffset>entry.source.length||range.endOffset<=range.startOffset||!Number.isFinite(range.count)||range.count<0)throw Error('coverage invalid V8 range: '+owner);
    }
    if(!entry.functions.some(fn=>fn.ranges.some(range=>range.startOffset===0&&range.endOffset===entry.source.length)))throw Error('coverage missing whole-program evidence: '+owner);
    let map=convertSourceMap.fromSource(entry.source)?.toObject(), mappedLines;
    let trace;
    if(map){
      const source=map.sources?.[0];
      const candidate=source?.startsWith('file://')?fileURLToPath(source):
        (isAbsolute(source??'')&&inventory.files[source]?source:resolve(dirname(owner),map.sourceRoot??'',source??''));
      if(map.sources?.length!==1||candidate!==owner)throw Error('coverage original owner mismatch: '+owner);
      if(map.sourcesContent?.[0]!==original.source||hash(map.sourcesContent[0])!==original.sha256)throw Error('coverage original source mismatch: '+owner);
      trace=new TraceMap(map);
      const originalRows=original.source.split('\n'),generatedRows=entry.source.split('\n');
      const decoded=decodedMappings(trace);
      for(const [line,row] of decoded.entries())for(const segment of row){
        if(line>=generatedRows.length||segment[0]>generatedRows[line].length||segment[0]<0||
          (segment.length>=4&&(segment[1]!==0||segment[2]<0||segment[2]>=originalRows.length||segment[3]<0||segment[3]>originalRows[segment[2]].length)))throw Error('coverage invalid source mapping: '+owner);
      }
      mappedLines=new Set(decoded.flatMap(row=>row.filter(segment=>segment.length>=4).map(segment=>segment[2]+1)));
      if(!mappedLines.size)throw Error('coverage unusable mappings: '+owner);
    }else if(entry.source!==original.source||hash(entry.source)!==original.sha256){
      throw Error('coverage missing source map: '+owner);
    }
    const converter=v8ToIstanbul(owner,0,{source:entry.source,...(map?{sourceMap:{sourcemap:map}}:{})});
    await converter.load();converter.applyCoverage(entry.functions);
    const converted=converter.toIstanbul();istanbul.push(converted);
    const file=converted[owner];
    if(!file)throw Error('coverage converter omitted owner: '+owner);
    const measured={};
    for(const [id,location] of Object.entries(file.statementMap)){
      const line=location.start.line;
      if(!mappedLines||mappedLines.has(line))measured[line]=Math.max(measured[line]??0,file.s[id]);
    }
    if(!Object.keys(measured).length)throw Error('coverage no measured original lines: '+owner);
    const unmappedStarts=trace?entry.functions.flatMap(fn=>fn.ranges).filter(range=>{
      const prefix=entry.source.slice(0,range.startOffset), rows=prefix.split('\n');
      return originalPositionFor(trace,{line:rows.length,column:rows.at(-1).length}).source===null;
    }).length:0;
    generated.push({url:entry.url,unmappedRangeStarts:unmappedStarts});
    // Query variants are separate generated programs, attributed only after
    // each original has independently matched the same inventoried bytes.
    const previous=lines[owner]??{};
    for(const [line,count] of Object.entries(measured))previous[line]=(previous[line]??0)+count;
    lines[owner]=previous;
  }
  return {runId:inventory.runId,lines,istanbul,generated,neverLoaded:selectedOwners(inventory).filter(path=>!lines[path]).sort()};
}

export function mergeReports(inventory,reports){
  const lines={};
  for(const report of reports){
    if(report.runId!==inventory.runId)throw Error('coverage foreign run');
    for(const [path,counts] of Object.entries(report.lines)){
      lines[path]??={};for(const [line,count] of Object.entries(counts))lines[path][line]=(lines[path][line]??0)+count;
    }
  }
  return {runId:inventory.runId,lines,neverLoaded:selectedOwners(inventory).filter(path=>!lines[path]).sort()};
}

export function requireReceipts(runId,selected){
  if(!selected.length)throw Error('coverage no selected tests');
  const receipts=selected.filter(test=>test.status!=='skipped').map(test=>{
    if(!test.receipt)throw Error('coverage missing test receipt: '+test.id);
    if(test.receipt.runId!==runId)throw Error('coverage foreign run: '+test.id);
    if(test.receipt.testId!==test.id)throw Error('coverage wrong test receipt: '+test.id);
    if(test.status!=='passed')throw Error('coverage unsuccessful test: '+test.id);
    return test.receipt;
  });
  if(!receipts.length)throw Error('coverage no completed receipts');
  return receipts;
}

async function inventoryFiles(){
  const files={};
  async function walk(path){
    for(const entry of await readdir(path,{withFileTypes:true})){
      const child=join(path,entry.name);
      if(entry.isDirectory())await walk(child);
      else if(/\.(mjs|jsx|js)$/.test(entry.name)&&!entry.name.endsWith('.test.mjs')){
        const source=await readFile(child,'utf8');files[child]={source,sha256:hash(source)};
      }
    }
  }
  for(const dir of ['components','shell','engine','catalog','vendor','tests/coverage-fixtures'])await walk(join(root,dir));
  for(const path of [join(root,'main.jsx'),join(root,'TriggerController.jsx')]){
    const source=await readFile(path,'utf8');files[path]={source,sha256:hash(source)};
  }
  return files;
}

async function run(){
  const directory=resolve(process.env.TRIGGER_COVERAGE_DIR??join(root,'coverage/browser',randomUUID()));
  await mkdir(directory,{recursive:true});
  if((await readdir(directory)).length)throw Error('coverage run directory is not empty: '+directory);
  const files=await inventoryFiles();
  const inventory={runId:randomUUID(),root,files,browserOwners:Object.keys(files).filter(path=>path.endsWith('.jsx'))};
  const inventoryPath=join(directory,'inventory.json');
  await writeFile(inventoryPath,JSON.stringify(inventory));
  const result=spawnSync(process.execPath,[join(root,'node_modules/@playwright/test/cli.js'),'test'],{
    cwd:root,stdio:'inherit',env:{...process.env,TRIGGER_BROWSER_COVERAGE:inventoryPath,TRIGGER_COVERAGE_DIR:directory},
  });
  if(result.error)throw result.error;
  const report=JSON.parse(await readFile(join(directory,'playwright.json'),'utf8')),selected=[];
  async function visit(suite){
    for(const child of suite.suites??[])await visit(child);
    for(const spec of suite.specs??[])for(const test of spec.tests)for(const result of test.results){
      const attachment=result.attachments.find(item=>item.name==='browser-coverage');
      selected.push({id:spec.id,status:result.status,receipt:attachment?JSON.parse(await readFile(attachment.path,'utf8')):undefined});
    }
  }
  await visit(report);
  const receipts=requireReceipts(inventory.runId,selected);
  if(result.status!==0)throw Error('coverage browser run failed: '+result.status);
  for(const [path,original] of Object.entries(inventory.files))if(hash(await readFile(path,'utf8'))!==original.sha256)throw Error('coverage owner changed during run: '+path);
  const reports=[];for(const capture of receipts)reports.push(await convertCapture(inventory,capture));
  const merged=mergeReports(inventory,reports);
  const istanbul=(await import('istanbul-lib-coverage')).default.createCoverageMap({});
  for(const report of reports)for(const converted of report.istanbul)istanbul.merge(converted);
  await writeFile(join(directory,'istanbul.json'),JSON.stringify(istanbul.toJSON()));
  await writeFile(join(directory,'generated.json'),JSON.stringify(reports.map(report=>report.generated)));
  const calibration=join(root,'tests/coverage-fixtures/callbacks.jsx');
  const lineOf=text=>inventory.files[calibration].source.split('\n').findIndex(line=>line.includes(text))+1;
  if(!(merged.lines[calibration]?.[lineOf("target.textContent = 'hit 😀'")]>0)||merged.lines[calibration]?.[lineOf("return 'never'")]!==0||merged.lines[calibration]?.[lineOf("return 'other'")]!==0||!merged.neverLoaded.includes(join(root,'tests/coverage-fixtures/unloaded.jsx')))throw Error('coverage real JSX calibration failed');
  const isCalibration=path=>path.includes('/tests/coverage-fixtures/');
  const counts=Object.entries(merged.lines).filter(([path])=>!isCalibration(path)).flatMap(([,lines])=>Object.values(lines));
  const summary={...merged,neverLoaded:merged.neverLoaded.filter(path=>!isCalibration(path)),unmeasuredByBrowser:Object.keys(files).filter(path=>!inventory.browserOwners.includes(path)),calibration:{passed:true,neverLoaded:merged.neverLoaded.filter(isCalibration)},selectedTests:selected.length,completedReceipts:receipts.length,mappedOriginalLines:counts.length,hitMappedOriginalLines:counts.filter(count=>count>0).length,
    scope:'Observed source-mapped JSX original line counts; never-loaded JSX owners and owners outside this browser profile are separate. Node V8 measures the .mjs unit-test scope. Vite plain-JS import rewriting does not supply reliable original-column maps. Raw Istanbul function/branch attribution includes transformed code and is not original-source function/branch coverage. Generated range starts are separate. Chromium development build, not native WebKit.'};
  await writeFile(join(directory,'summary.json'),JSON.stringify(summary,null,2)+'\n');
  console.log(`Browser coverage: ${receipts.length} test receipts, ${Object.keys(merged.lines).filter(path=>!isCalibration(path)).length} observed owners, ${summary.hitMappedOriginalLines}/${counts.length} mapped original lines; ${summary.neverLoaded.length} never loaded.`);
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url))run().catch(error=>{console.error(error);process.exitCode=1;});
