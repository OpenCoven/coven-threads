const theme = document.querySelector('#theme');
try { const saved = localStorage.getItem('threads-theme'); if (saved) document.documentElement.dataset.theme = saved; } catch {}
theme.addEventListener('click', () => {const next = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';document.documentElement.dataset.theme=next;try{localStorage.setItem('threads-theme',next);}catch{}});
const dialog = document.querySelector('#search-dialog');
const input = document.querySelector('#search-input');
const results = document.querySelector('#search-results');
let index;
async function find() {
  results.replaceChildren();
  const q = input.value.trim().toLowerCase();
  if (!q) {const p=document.createElement('p');p.textContent='Search concepts, contracts and delivery evidence.';results.append(p);return;}
  try {
    index ??= await fetch('/search.json').then(r=>{if(!r.ok)throw Error('Search unavailable');return r.json();});
    const matches=index.filter(p=>(p.title+' '+p.text).toLowerCase().includes(q)).slice(0,12);
    if(q!==input.value.trim().toLowerCase())return;
    if(!matches.length){const p=document.createElement('p');p.textContent='No pages found. Try a different term.';results.append(p);}
    for(const p of matches){const a=document.createElement('a');a.href=p.url;a.textContent=p.title;results.append(a);}
  }catch{results.textContent='Search could not load. Use the documentation menu to browse.';}
}
function openSearch(){if(!dialog.open)dialog.showModal();input.focus();find();}
document.querySelector('[data-search]').addEventListener('click',openSearch);
document.querySelector('#close-search').addEventListener('click',()=>dialog.close());
document.addEventListener('keydown',e=>{if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==='k'){e.preventDefault();openSearch();}});
input.addEventListener('input',find);
for (const pre of document.querySelectorAll('pre')) {
  const button=document.createElement('button');button.className='copy-code';button.textContent='Copy';button.type='button';
  button.addEventListener('click',async()=>{try{await navigator.clipboard.writeText(pre.querySelector('code').textContent);button.textContent='Copied';}catch{button.textContent='Select text to copy';}});pre.prepend(button);
}
