import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { qualifyTerminalCaptures } from "../lib/daemon-window-terminals.mjs";

function fixture(t) {
  const root=mkdtempSync(join(tmpdir(),"threads-window-terminals-"));
  t.after(()=>rmSync(root,{recursive:true,force:true}));
  const dir=join(root,"synthetic-journey");mkdirSync(join(dir,"state"),{recursive:true});
  writeFileSync(join(dir,"manifest.json"),JSON.stringify({scenario:"explicit-supersession",result:"passed"}));
  const windows=[{proposalId:"synthetic-one",classification:"typed_terminal_recorded",closeReason:"superseded"},{proposalId:"synthetic-two",classification:"typed_terminal_recorded",closeReason:"applied"}];
  const rows=[
    {id:1,event_type:"proposal_window_opened",proposal_id:"synthetic-one",detail:null},
    {id:2,event_type:"proposal_rejected",proposal_id:"synthetic-one",detail:JSON.stringify({reason:"superseded",replay_hash_matched:null,rationale:null})},
    {id:3,event_type:"proposal_window_opened",proposal_id:"synthetic-two",detail:null},
    {id:4,event_type:"proposal_approved",proposal_id:"synthetic-two",detail:JSON.stringify({window_close:{reason:"applied",replay_hash_matched:true,rationale:null}})},
  ];
  const save=()=>{writeFileSync(join(dir,"state/ward-window-census.json"),JSON.stringify({complete:true,windows}));writeFileSync(join(dir,"state/ward-audit.jsonl"),rows.map(r=>JSON.stringify(r)).join("\n")+"\n");};save();
  return {root,dir,windows,rows,save};
}
test("scoped terminal captures prove every retained opened window has one typed close",t=>{
  const f=fixture(t),p=qualifyTerminalCaptures(f.root);
  assert.equal(p.qualified,true);assert.equal(p.windows,2);assert.equal(p.captures,1);
  assert.equal(p.missing,0);assert.equal(p.duplicates,0);assert.equal(p.sources[0].audit_sha256.length,64);
});
for(const kind of ["wrong-family","bad-replay","bad-rationale","close-before-open"]) test(`canonical terminal rule refuses ${kind}`,t=>{
  const f=fixture(t);
  if(kind === "wrong-family") {f.rows[1].event_type="proposal_vetoed";f.rows[3].event_type="proposal_vetoed";f.rows[3].detail=JSON.stringify({reason:"applied",replay_hash_matched:true});}
  if(kind === "bad-replay") f.rows[3].detail=JSON.stringify({window_close:{reason:"applied",replay_hash_matched:"not-a-boolean"}});
  if(kind === "bad-rationale") f.rows[1].detail=JSON.stringify({reason:"superseded",replay_hash_matched:null,rationale:{invalid:"object"}});
  if(kind === "close-before-open") {f.rows[0].id=2;f.rows[1].id=1;}
  f.save();assert.throws(()=>qualifyTerminalCaptures(f.root));
});
for(const kind of ["missing-close","duplicate-close","missing-open","duplicate-open","duplicate-id","bad-classification","bad-reason","missing-census","incomplete","missing-required","missing-window"]) test(`terminal proof refuses ${kind}`,t=>{
  const f=fixture(t);
  if(kind === "missing-close") f.rows.splice(1,1);
  if(kind === "duplicate-close") f.rows.push({...f.rows[1],id:5});
  if(kind === "missing-open") f.rows.splice(0,1);
  if(kind === "duplicate-open") f.rows.push({...f.rows[0],id:5});
  if(kind === "duplicate-id") f.rows[1].id=1;
  if(kind === "bad-classification") f.windows[0].classification="unresolved";
  if(kind === "bad-reason") f.windows[0].closeReason="wrong";
  if(kind === "missing-window") f.windows.splice(0,1);
  f.save();
  if(kind === "missing-census") rmSync(join(f.dir,"state/ward-window-census.json"));
  if(kind === "incomplete") {const p=join(f.dir,"state/ward-window-census.json");const x=JSON.parse(readFileSync(p));x.complete=false;writeFileSync(p,JSON.stringify(x));}
  if(kind === "missing-required") writeFileSync(join(f.dir,"manifest.json"),JSON.stringify({scenario:"other",result:"passed"}));
  assert.throws(()=>qualifyTerminalCaptures(f.root));
});
