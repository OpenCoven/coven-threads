import assert from "node:assert/strict";
import {mkdirSync,mkdtempSync,readFileSync,readdirSync,rmSync,writeFileSync} from "node:fs";
import {tmpdir} from "node:os";
import {join} from "node:path";
import test from "node:test";
import {collectWindow,authenticateRetainedArchive} from "../qualify-daemon-window.mjs";
import {verifyArtifactIndex} from "../lib/daemon-artifact-archive.mjs";
import {syntheticWindowControl} from "./fixtures/native-window-control.mjs";

function fixture(t,count=0) {
  const root=mkdtempSync(join(tmpdir(),"threads-window-collector-"));t.after(()=>rmSync(root,{recursive:true,force:true}));
  const f=syntheticWindowControl(),declaration=join(root,"declaration.json"),archive=join(root,"archive"),output=join(root,"output");
  writeFileSync(declaration,JSON.stringify(f.declaration));mkdirSync(archive);
  const runs=Array.from({length:count},(_,n)=>({...f.runs[n%30],id:n+1}));
  const calls=[];let pass=0;
  const read=endpoint=>{
    calls.push(endpoint);const page=Number(new URL("https://api.example.invalid/"+endpoint).searchParams.get("page"));if(page===1) pass++;
    const binding=Object.fromEntries(["schema_version","repository","workflow","window_start","days","schedule_utc","governing_metric","minimum_percent","scenario_inventory_sha256"].map(k=>[k,f.declaration[k]]));
    const body=endpoint.includes("/issues/comments/")?{id:42,issue_url:"https://api.github.com/repos/OpenCoven/coven-threads/issues/111",created_at:f.declaration.declared_at,updated_at:f.declaration.declared_at,user:{login:"BunsDev"},body:JSON.stringify(binding)}:{total_count:runs.length,workflow_runs:runs.slice((page-1)*100,page*100)};
    return Buffer.from("HTTP/2.0 200 OK\r\nDate: Sun, 01 Feb 2026 00:00:00 GMT\r\nContent-Type: application/json\r\n\r\n"+JSON.stringify(body));
  };
  return {root,declaration,archive,output,read,calls,runs,get pass(){return pass;}};
}
test("stable paginated complete census retains all responses and does not invent archives",t=>{
  const f=fixture(t,101),r=collectWindow(f.declaration,f.archive,f.output,f.read);
  assert.equal(f.calls.length,5);assert.equal(r.ledger.expected_os_observations,90);assert.equal(r.ledger.qualified_os_observations,0);assert.equal(r.ledger.window_qualified,false);assert.equal(r.ledger.predeclaration_verified,true);
  assert.equal(readdirSync(join(f.output,"metadata")).length,5);verifyArtifactIndex(f.output);
  assert.match(readFileSync(join(f.output,"packet.md"),"utf8"),/90/);
  assert.throws(()=>collectWindow(f.declaration,f.archive,f.output,f.read),/exist/i);
});
for(const kind of ["edited","wrong-window","wrong-owner"]) test(`predeclaration provenance refuses ${kind}`,t=>{
  const f=fixture(t),read=f.read;
  const bad=endpoint=>{
    const bytes=read(endpoint);
    if(!endpoint.includes("/issues/comments/")) return bytes;
    let text=bytes.toString();
    if(kind === "edited") text=text.replace('"updated_at":"2025-12-31T00:00:00Z"','"updated_at":"2026-02-01T00:00:00Z"');
    if(kind === "wrong-window") text=text.replaceAll("2026-01-01","2026-01-02");
    if(kind === "wrong-owner") text=text.replace("BunsDev","synthetic-other");
    return Buffer.from(text);
  };
  assert.throws(()=>collectWindow(f.declaration,f.archive,f.output,bad));verifyArtifactIndex(f.output);
});

test("live archive reauthentication refuses changed run/job/artifact bindings",t=>{
  const f=fixture(t),run={id:7,run_attempt:1,head_sha:"a".repeat(40),event:"schedule",path:".github/workflows/daemon-reliability.yml",repository:{full_name:"OpenCoven/coven-threads"},status:"completed",conclusion:"success",created_at:"2026-01-01T07:47:00Z",run_started_at:"2026-01-01T07:48:00Z"};
  const job={name:"Native daemon / ubuntu-24.04",id:9,conclusion:"success"};
  const artifact={id:11,name:"synthetic-original",digest:"sha256:"+"b".repeat(64),size_in_bytes:3,workflow_run:{id:7,head_sha:run.head_sha}};
  const record={run_id:7,attempt:1,threads_sha:run.head_sha,rows:[{os:"ubuntu-24.04",job,artifact}]};
  const dir=join(f.archive,"7/1/metadata");mkdirSync(dir,{recursive:true});writeFileSync(join(dir,"run.json"),JSON.stringify(run));
  for(const kind of ["valid","run","job","artifact"]) {
    const read=(_name,endpoint)=>{
      if(endpoint.includes("/git/commits/")) return {sha:run.head_sha,tree:{sha:"c".repeat(40)}};
      if(endpoint.includes("/jobs?")) return {total_count:1,jobs:[{...job,conclusion:kind === "job"?"failure":"success"}]};
      if(endpoint.includes("/artifacts/")) return {...artifact,digest:kind === "artifact"?"sha256:"+"c".repeat(64):artifact.digest};
      return {...run,conclusion:kind === "run"?"failure":"success"};
    };
    if(kind === "valid") assert.equal(authenticateRetainedArchive(f.archive,[record],read)[run.head_sha],"c".repeat(40));
    else assert.throws(()=>authenticateRetainedArchive(f.archive,[record],read));
  }
});

test("read failure retains original subprocess diagnostics in indexed output",t=>{
  const f=fixture(t);
  const error=new Error("synthetic read failure");error.diagnostic={status:73,stdout_base64:Buffer.from("original stdout").toString("base64"),stderr_base64:Buffer.from("original stderr").toString("base64")};
  assert.throws(()=>collectWindow(f.declaration,f.archive,f.output,()=>{throw error;}));
  const name=readdirSync(join(f.output,"metadata")).find(n=>n.endsWith("read-failure.json"));assert.ok(name);
  assert.equal(JSON.parse(readFileSync(join(f.output,"metadata",name))).status,73);verifyArtifactIndex(f.output);
});
for(const kind of ["changed-snapshot","changing-total","bound","missing-date","backwards-date","bad-status","unindexed-archive","bad-page"]) test(`collection preserves failure and refuses ${kind}`,t=>{
  const f=fixture(t,101),read=f.read;
  const bad=endpoint=>{
    let text=read(endpoint).toString();
    if(kind === "changed-snapshot" && f.pass>1) text=text.replace('"conclusion":"success"','"conclusion":"failure"');
    if(kind === "changing-total" && endpoint.endsWith("page=2")) text=text.replace('"total_count":101','"total_count":102');
    if(kind === "bound") text=text.replace('"total_count":101','"total_count":1000');
    if(kind === "missing-date") text=text.replace(/Date: [^\r]+\r\n/,"");
    if(kind === "backwards-date" && f.calls.length>1) text=text.replace("Sun, 01 Feb 2026","Sat, 31 Jan 2026");
    if(kind === "bad-status") text=text.replace("200 OK","403 Forbidden");
    if(kind === "bad-page" && endpoint.endsWith("page=2")) text=text.replace(/"workflow_runs":\[.*\]/,'"workflow_runs":[]');
    return Buffer.from(text);
  };
  if(kind === "unindexed-archive") writeFileSync(join(f.archive,"unknown"),"unindexed");
  assert.throws(()=>collectWindow(f.declaration,f.archive,f.output,bad));
  verifyArtifactIndex(f.output);assert.ok(JSON.parse(readFileSync(join(f.output,"failure.json"))).qualified===false);
});
