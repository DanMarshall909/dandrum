import {existsSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const cwd=fileURLToPath(new URL('../',import.meta.url));
if(!existsSync(cwd+'node_modules/vite/package.json')){
  console.error('Trigger checks unavailable: run npm ci in web/trigger before CTest.');process.exit(1);
}
for(const name of ['check','test','build','test:browser']){
  const result=spawnSync('npm',['run',name],{cwd,stdio:'inherit',shell:process.platform==='win32'});
  if(result.error){console.error('Trigger '+name+' could not run: '+result.error.message);process.exit(1);}
  if(result.status!==0)process.exit(result.status??1);
}
