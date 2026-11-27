import { readFile, writeFile, mkdir, cp, rm } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Marked, Renderer } from 'marked';
import GithubSlugger from 'github-slugger';
const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.dirname(here);
const out = path.join(here, 'dist');
const pages = JSON.parse(await readFile(path.join(here, 'pages.json'), 'utf8'));
const revision = process.env.VERCEL_GIT_COMMIT_SHA || execFileSync('git', ['rev-parse', 'HEAD'], {cwd: root, encoding: 'utf8'}).trim();
const source = `https://github.com/OpenCoven/coven-threads/blob/${revision}/`;
const escape = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const routes = new Map(pages.map(p => [p.file, `/docs/${p.slug}`]));
await rm(out, {recursive:true, force:true});
await mkdir(out, {recursive:true});
await cp(path.join(here,'assets'), path.join(out,'assets'), {recursive:true});
const search = [];
function nav(active) {
  let last = '';
  return `<a href="/" ${active === '/' ? 'aria-current="page"' : ''}>Overview</a>` + pages.map(p => {
    const heading = p.group === last ? '' : `<p class="nav-label">${escape(p.group)}</p>`; last = p.group;
    return `${heading}<a href="/docs/${p.slug}" ${active === `/docs/${p.slug}` ? 'aria-current="page"' : ''}>${escape(p.title)}</a>`;
  }).join('');
}
function shell(title, route, body, toc = [], file = null) {
  const index = pages.findIndex(p => routes.get(p.file) === route);
  const adjacent = index < 0 ? [] : [pages[index-1], pages[index+1]].filter(Boolean);
  return `<!doctype html><html lang="en" data-theme="dark"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta name="color-scheme" content="dark light"><title>${escape(title)} · Coven Threads</title><meta name="description" content="Typed authority contracts for protected familiar surfaces. The official Coven Threads documentation."><link rel="icon" href="/assets/sigil.svg" type="image/svg+xml"><link rel="stylesheet" href="/assets/style.css"><script src="/assets/site.js" defer></script></head><body><a class="skip" href="#main">Skip to content</a><header><a class="brand" href="/"><img src="/assets/sigil.svg" width="30" height="30" alt=""><span>Coven <strong>Threads</strong></span><span class="badge">Docs</span></a><nav aria-label="Ecosystem"><a href="https://docs.opencoven.ai">Coven Docs ↗</a><a href="https://github.com/OpenCoven/coven-threads">GitHub ↗</a><button id="theme" type="button" aria-label="Switch color theme">◐</button></nav></header><div class="layout"><aside class="sidebar"><button class="search-button" type="button" data-search>Search documentation <kbd>⌘ K</kbd></button><details id="mobile-nav"><summary>Documentation menu</summary><nav aria-label="Mobile documentation">${nav(route)}</nav></details><nav class="desktop-nav" aria-label="Documentation">${nav(route)}</nav><div class="sidebar-foot"><span class="status-dot"></span> Protected-authority validator<p>Contracts here. Effects in Coven.</p></div></aside><main id="main" tabindex="-1"><div class="page-meta">${file ? 'DOCUMENTATION' : 'THE AUTHORITY LAYER'} <span>coven-threads-core · 0.2</span></div>${body}<div class="page-bottom">${adjacent.map(p=>`<a href="/docs/${p.slug}">${escape(p.title)} <span>→</span></a>`).join('')}</div><footer>${file ? `<a href="${source}${file}">View canonical source ↗</a>` : '<a href="/docs/concepts">Start with concepts →</a>'}<span>Source snapshot <a href="https://github.com/OpenCoven/coven-threads/tree/${revision}">${revision.slice(0,8)}</a></span><p>These docs explain the contracts; frozen specs and public types retain normative authority.</p></footer></main><aside class="toc" aria-label="On this page"><p>On this page</p>${toc.map(h=>`<a class="depth-${h.level}" href="#${h.id}">${escape(h.text)}</a>`).join('')}<a class="toc-source" href="${file ? source+file : 'https://github.com/OpenCoven/coven-threads'}">${file ? 'Read source' : 'Explore the repository'} ↗</a></aside></div><dialog id="search-dialog" aria-labelledby="search-label"><div class="dialog-top"><label id="search-label" for="search-input">Search Threads</label><button id="close-search" type="button" aria-label="Close search">Esc</button></div><input id="search-input" type="search" placeholder="Concepts, channels, authority…" autocomplete="off"><div id="search-results" aria-live="polite"></div></dialog></body></html>`;
}
for (const p of pages) {
  const markdown = await readFile(path.join(root,p.file),'utf8');
  const slugger = new GithubSlugger(); const toc = [];
  const parser = new Marked({gfm:true});
  function resolve(href, image=false) {
    if (/^(https?:|mailto:|#)/.test(href)) return href;
    const [relative, fragment] = href.split('#');
    const resolved = path.posix.normalize(path.posix.join(path.posix.dirname(p.file),relative));
    if (!resolved.startsWith('../') && routes.has(resolved)) return routes.get(resolved)+(fragment?'#'+fragment:'');
    return (image ? `https://raw.githubusercontent.com/OpenCoven/coven-threads/${revision}/` : source)+resolved+(fragment?'#'+fragment:'');
  }
  parser.use({renderer:{
    heading({tokens,depth}) {
      const html = this.parser.parseInline(tokens); const text = html.replace(/<[^>]*>/g,''); const id=slugger.slug(text.replace(/&#39;|&apos;/g, "'").replace(/&quot;/g, '"').replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>'));
      if(depth===2 || depth===3) toc.push({id,text,level:depth});
      return `<h${depth} id="${escape(id)}"><a class="heading-anchor" href="#${escape(id)}">${html}</a></h${depth}>`;
    },
    link({href,title,tokens}) {return `<a href="${escape(resolve(href))}"${title?` title="${escape(title)}"`:''}>${this.parser.parseInline(tokens)}</a>`;},
    image({href,text}) {return `<img src="${escape(resolve(href,true))}" alt="${escape(text)}" loading="lazy">`;},
    table(token) {return `<div class="table-scroll">${new Renderer().table.call(this,token)}</div>`;}
  }});
  const html = parser.parse(markdown);
  await mkdir(path.join(out,'docs',path.dirname(p.slug)),{recursive:true});
  await writeFile(path.join(out,`docs/${p.slug}.html`),shell(p.title, routes.get(p.file),`<article>${html}</article>`,toc,p.file));
  search.push({title:p.title,url:routes.get(p.file),text:markdown.replace(/[#*`|>\[\]]/g,'').replace(/\s+/g,' ')});
}
const home = `<section class="hero"><p class="eyebrow">OpenCoven / Threads</p><h1>Bind the authority.<br><em>Keep the weave.</em></h1><p class="lead">Typed contracts for the surfaces that make a familiar who they are. Threads validates protected mutations inside the trusted Coven daemon.</p><div class="hero-actions"><a class="primary" href="/docs/concepts">Read the concepts <span>→</span></a><a href="/docs/architecture">Explore the architecture ↗</a></div><div class="boundary"><span>Familiar / client</span><b>→</b><span class="trusted">Coven daemon <small>Threads validator</small></span><b>→</b><span>Protected surface</span></div><p class="caption">The daemon authenticates and owns effects. Threads supplies the typed verdict.</p></section><section class="intro-grid" aria-label="Documentation paths"><a href="/docs/authority-model"><span class="eyebrow">01 / Validate</span><h2>Authority, made explicit.</h2><p>Permit, degrade to proposal, or reject. Understand how a bound writer, covered channel, strands and predicate contribute to a verdict.</p><span class="card-link">Authority model →</span></a><a href="/docs/channels-and-strands"><span class="eyebrow">02 / Preserve</span><h2>Know what must hold.</h2><p>Threads bind surfaces to writers. Strands carry commitments. Channels describe the load a weave must survive.</p><span class="card-link">Channels & strands →</span></a><a href="/docs/automation-authority-profile"><span class="eyebrow">03 / Integrate</span><h2>Carry the evidence.</h2><p>Explore the versioned automation authority profile, operation-specific decisions, schemas and portable conformance vectors.</p><span class="card-link">Automation authority →</span></a><a href="/docs/phases"><span class="eyebrow">04 / Verify</span><h2>Read the delivery ledger.</h2><p>Design freezes, source provenance, daemon proof and human acceptance are recorded separately, with their dates and limits intact.</p><span class="card-link">Status & evidence →</span></a></section><aside class="notice"><strong>The trust boundary stays in Coven.</strong><p>Threads does not issue credentials, write protected files, own persistence, or grant authority to a prompt or descriptor. Unknown or ambiguous authority fails closed.</p></aside>`;
await writeFile(path.join(out,'index.html'),shell('Protected-authority documentation','/',home));
await writeFile(path.join(out,'404.html'),shell('Page not found','/404','<h1>That thread ends here.</h1><p>This page could not be found. <a href="/">Return to the documentation.</a></p>'));
await writeFile(path.join(out,'search.json'),JSON.stringify(search));
await writeFile(path.join(out,'provenance.json'),JSON.stringify({repository:'OpenCoven/coven-threads',revision,pages:pages.map(p=>p.file)},null,2));
const config = JSON.parse(await readFile(path.join(here,'vercel.json'),'utf8'));
await writeFile(path.join(out,'vercel.json'),JSON.stringify({cleanUrls:true,headers:config.headers},null,2));
console.log(`Built ${pages.length+1} pages from ${revision}`);
