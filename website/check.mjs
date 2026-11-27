import { readFile, access } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root=path.join(path.dirname(fileURLToPath(import.meta.url)),'dist');
const pages=JSON.parse(await readFile(new URL('./pages.json',import.meta.url)));
const files=['index.html',...pages.map(p=>`docs/${p.slug}.html`),'404.html'];
let links=0,fragments=0;
for(const file of files){
 const html=await readFile(path.join(root,file),'utf8');
 for(const match of html.matchAll(/(?:href|src)="([^"]+)"/g)){
  const url=match[1];if(!url.startsWith('/')&&!url.startsWith('#'))continue;
  const [route,fragment]=url.split('#');let target=route?route.slice(1):file;
  if(!target)target='index.html';else if(!path.extname(target))target+='.html';
  await access(path.join(root,target));links++;
  if(fragment){const body=target===file?html:await readFile(path.join(root,target),'utf8');if(!body.includes(`id="${fragment}"`))throw Error(`${file}: missing ${url}`);fragments++;}
 }
 if(!html.includes('Skip to content')||!html.includes('Source snapshot'))throw Error(`${file}: missing reading/source contract`);
 if(/(?:href|src)="(?:javascript:|file:|\/Users\/)/i.test(html))throw Error(`${file}: unsafe or private URL`);
}
console.log(`PASS: ${files.length} documents, ${links} local links/assets, ${fragments} fragments; source and accessibility structure present.`);
