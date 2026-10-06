import {readdir,readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
import {transformWithEsbuild} from 'vite';
const root=process.argv[2]?resolve(process.argv[2])+'/':fileURLToPath(new URL('../',import.meta.url));
async function files(directory){
  const entries=await readdir(root+directory,{withFileTypes:true});
  return (await Promise.all(entries.map(entry=>entry.isDirectory()?files(directory+'/'+entry.name):entry.name.endsWith('.jsx')?[directory+'/'+entry.name]:[]))).flat();
}
const paths=['TriggerController.jsx',...await files('components'),...await files('catalog')];
const violations=[];let largest={path:'',lines:0};
for(const path of paths){
  const source=await readFile(root+path,'utf8'),lines=source.trimEnd().split('\n').length;
  if(lines>largest.lines)largest={path,lines};
  if(lines>=200)violations.push(`${path}: ${lines} lines (target under 200)`);
  if(path.startsWith('components/')&&/from\s+['"][^'"]*engine\//.test(source))violations.push(`${path}: UI component imports the engine; pass values and callbacks`);
  try{await transformWithEsbuild(source,path,{loader:'jsx'});}catch(error){violations.push(`${path}: invalid JSX: ${error.errors?.[0]?.text??error.message}`);}
}
if(violations.length){console.error(violations.join('\n'));process.exitCode=1;}
else console.log(`${paths.length} authored React files checked; largest ${largest.path}: ${largest.lines} lines.`);
