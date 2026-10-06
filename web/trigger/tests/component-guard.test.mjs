import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
test('component convention guard names oversized files, engine imports and broken JSX instead of treating setup errors as failures',async()=>{
 const root=await mkdtemp(tmpdir()+'/trigger-guard-'),script=fileURLToPath(new URL('../scripts/check-components.mjs',import.meta.url));
 try{await mkdir(root+'/components');await mkdir(root+'/catalog');await writeFile(root+'/TriggerController.jsx','export function TriggerController(){return <div/>;}');
 const check=()=>{try{return {status:0,text:execFileSync(process.execPath,[script,root],{encoding:'utf8',stdio:'pipe'})};}catch(error){return {status:error.status,text:error.stderr.toString()};}};
 assert.equal(check().status,0);
 for(const [source,diagnostic] of [['// line\n'.repeat(200),'components/Fault.jsx: 200 lines'],["import {Store} from '../engine/store.mjs';",'UI component imports the engine'],['export const Fault = () => <div><span></div>;','invalid JSX']]){
  await writeFile(root+'/components/Fault.jsx',source);const result=check();assert.equal(result.status,1);assert.ok(result.text.includes(diagnostic),result.text);await rm(root+'/components/Fault.jsx');assert.equal(check().status,0);
 }
 }finally{await rm(root,{recursive:true,force:true});}
});
