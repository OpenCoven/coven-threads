#!/usr/bin/env node
import {spawnSync,execFileSync} from "node:child_process";
import {lstatSync,mkdirSync,readFileSync,writeFileSync} from "node:fs";
import {dirname,join,relative,resolve,sep} from "node:path";
import {fileURLToPath} from "node:url";
import {createHash} from "node:crypto";
import {isDeepStrictEqual} from "node:util";
import {readRetainedArchive} from "./retain-daemon-evidence.mjs";
import {writeArtifactIndex} from "./lib/daemon-reliability-evidence.mjs";
import {buildWindowLedger,validateWindowDeclaration} from "./lib/daemon-window-ledger.mjs";

const repository="OpenCoven/coven-threads",hash=bytes=>createHash("sha256").update(bytes).digest("hex");
const json=value=>JSON.stringify(value,null,2)+"\n";
const regular=path=>{if(!lstatSync(path).isFile()) throw Error("window input must be a regular file");return readFileSync(path);};
function githubRead(endpoint) {
  const r=spawnSync("gh",["api","--include",endpoint],{encoding:null,maxBuffer:256*1024*1024});
  if(r.error || r.signal || r.status !== 0) {
    const error=Error("authenticated GitHub REST census read failed");
    error.diagnostic={program:"gh",args:["api","--include",endpoint],status:r.status,signal:r.signal ?? null,spawn_error:r.error ? {code:r.error.code ?? null,message:r.error.message}:null,stdout_base64:(r.stdout ?? Buffer.alloc(0)).toString("base64"),stderr_base64:(r.stderr ?? Buffer.alloc(0)).toString("base64")};
    throw error;
  }
  return r.stdout;
}
function response(bytes) {
  const text=bytes.toString("utf8"),split=text.search(/\r?\n\r?\n/);
  if(split<0) throw Error("missing HTTP response headers");
  const headers=text.slice(0,split).split(/\r?\n/),status=headers.shift();
  if(!/^HTTP\/[0-9.]+ 200(?: .*)?$/.test(status)) throw Error("GitHub read did not return HTTP 200");
  const dates=headers.filter(line=>/^date:/i.test(line)).map(line=>line.slice(5).trim());
  if(dates.length !== 1 || !Number.isFinite(Date.parse(dates[0])) || new Date(dates[0]).toUTCString() !== dates[0]) throw Error("one canonical GitHub HTTP Date is required");
  const body=text.slice(split).replace(/^\r?\n\r?\n/,"");
  return {date:Date.parse(dates[0]),body:JSON.parse(body)};
}
function pages(snapshot,prefix,endpoint,key,bound=1000) {
  const rows=[];let total;
  for(let page=1;page<=10;page++) {
    const data=snapshot(`${prefix}-${page}`,`${endpoint}${endpoint.includes("?")?"&":"?"}per_page=100&page=${page}`);
    if(!Number.isSafeInteger(data.total_count) || data.total_count<0 || data.total_count>=bound || !Array.isArray(data[key])) throw Error("invalid or search-bounded census");
    total ??= data.total_count;
    if(total !== data.total_count) throw Error("census total changed during pagination");
    rows.push(...data[key]);
    if(rows.length === total) return rows;
    if(rows.length>total || !data[key].length) throw Error("incomplete census pages");
  }
  throw Error("census exceeds pagination bound");
}

// Reauthenticate archived identities against live REST data; offline indexes alone are insufficient.
export function authenticateRetainedArchive(archiveRoot,records,snapshot) {
  const commits=new Map();
  for(const record of records) {
    const directory=join(archiveRoot,String(record.run_id),String(record.attempt),"metadata");
    const original=JSON.parse(regular(join(directory,"run.json")));
    const prefix=`repos/${repository}/actions/runs/${record.run_id}/attempts/${record.attempt}`;
    const current=snapshot(`attempt-${record.run_id}-${record.attempt}`,prefix);
    const identity=r=>({id:r.id,attempt:r.run_attempt,sha:r.head_sha,event:r.event,path:r.path,repository:r.repository?.full_name,status:r.status,conclusion:r.conclusion,created:r.created_at,started:r.run_started_at});
    if(!isDeepStrictEqual(identity(original),identity(current))) throw Error("live run identity/outcome contradicts archived attempt");
    const jobs=pages(snapshot,`jobs-${record.run_id}-${record.attempt}`,`${prefix}/jobs`,"jobs");
    for(const row of record.rows) {
      const matching=jobs.filter(j=>j.name===`Native daemon / ${row.os}`);
      if(matching.length>1 || !isDeepStrictEqual(matching[0] ?? null,row.job)) throw Error("live job contradicts original archived identity/outcome");
      if(row.artifact) {
        const artifact=snapshot(`artifact-${record.run_id}-${record.attempt}-${row.os}`,`repos/${repository}/actions/artifacts/${row.artifact.id}`);
        const binding=a=>({id:a.id,name:a.name,digest:a.digest,bytes:a.size_in_bytes,run:a.workflow_run?.id,source:a.workflow_run?.head_sha});
        if(!isDeepStrictEqual(binding(artifact),binding(row.artifact))) throw Error("live artifact digest/identity differs from original archive");
      }
    }
    if(!commits.has(record.threads_sha)) {
      const commit=snapshot(`source-${record.threads_sha}`,`repos/${repository}/git/commits/${record.threads_sha}`);
      if(commit.sha !== record.threads_sha || !/^[a-f0-9]{40}$/.test(commit.tree?.sha ?? "")) throw Error("source commit/tree could not be authenticated");
      commits.set(record.threads_sha,commit.tree.sha);
    }
  }
  return Object.fromEntries([...commits].sort());
}

function collectorIdentity() {
  const files=["qualify-daemon-window.mjs","retain-daemon-evidence.mjs","lib/daemon-window-ledger.mjs","lib/daemon-window-terminals.mjs","lib/daemon-retention.mjs","lib/daemon-artifact-archive.mjs","lib/extract-daemon-artifact.py","lib/daemon-reliability-evidence.mjs","../e2e/native-daemon-scenarios.json","../crates/coven-threads-core/src/approval.rs","../crates/coven-threads-core/src/audit.rs"];
  const root=fileURLToPath(new URL("..",import.meta.url));
  const git=args=>execFileSync("git",args,{cwd:root,encoding:"utf8"}).trim();
  const python=spawnSync("python3",["-I","-c","import sys; assert sys.version_info >= (3,11); print(sys.version.split()[0])"],{encoding:"utf8"});
  if(python.error || python.status !== 0) throw Error("Python 3.11+ is required");
  return {node:process.version,python:python.stdout.trim(),source_commit:git(["rev-parse","HEAD"]),source_tree:git(["rev-parse","HEAD^{tree}"]),source_clean:git(["status","--porcelain=v1","--untracked-files=all"]) === "",source_sha256:Object.fromEntries(files.map(path=>[path,hash(readFileSync(new URL(path,import.meta.url)))]))};
}

function authenticateDeclaration(declaration,start,snapshot) {
  const comment=snapshot("predeclaration",`repos/${repository}/issues/comments/${declaration.declaration_comment_id}`);
  if(comment.id !== declaration.declaration_comment_id || comment.issue_url !== `https://api.github.com/repos/${repository}/issues/111` || comment.user?.login !== "BunsDev" || comment.created_at !== comment.updated_at || Date.parse(comment.created_at) !== Date.parse(declaration.declared_at) || Date.parse(comment.created_at)>=start) throw Error("predeclaration comment identity/time is unverified or edited");
  const keys=["schema_version","repository","workflow","window_start","days","schedule_utc","governing_metric","minimum_percent","scenario_inventory_sha256"];
  const expected=Object.fromEntries(keys.map(key=>[key,declaration[key]]));
  const body=comment.body.trim(),fenced=body.match(/^```json\r?\n([\s\S]*?)\r?\n```$/);
  if(!isDeepStrictEqual(JSON.parse(fenced?fenced[1]:body),expected)) throw Error("predeclaration comment differs from window/denominator contract");
  return {id:comment.id,created_at:comment.created_at,account:comment.user.login,contract_sha256:hash(JSON.stringify(expected)),human_acceptance_inferred:false};
}

export function collectWindow(declarationPath,archiveRoot,output,read=githubRead) {
  output=resolve(output);archiveRoot=resolve(archiveRoot);
  const rel=relative(archiveRoot,output);
  if(!rel || (!rel.startsWith(`..${sep}`) && !rel.startsWith(sep))) throw Error("window output must be outside immutable native archives");
  mkdirSync(dirname(output),{recursive:true});mkdirSync(output);mkdirSync(join(output,"metadata"));
  let serial=0;const dates=[];
  const snapshot=(name,endpoint)=>{
    const prefix=`${String(++serial).padStart(4,"0")}-${name}`;
    let bytes;
    try {bytes=read(endpoint);}
    catch(error) {writeFileSync(join(output,"metadata",`${prefix}-read-failure.json`),json(error.diagnostic ?? {error:error.message}),{flag:"wx"});throw error;}
    writeFileSync(join(output,"metadata",`${prefix}.http`),bytes,{flag:"wx"});
    const parsed=response(bytes);
    if(dates.length && parsed.date<dates.at(-1)) throw Error("GitHub response clock moved backwards");
    dates.push(parsed.date);return parsed.body;
  };
  try {
    if(process.versions.node !== "24.22.0") throw Error("Node 24.22.0 is required");
    const declarationBytes=regular(declarationPath);writeFileSync(join(output,"declaration.json"),declarationBytes,{flag:"wx"});
    const declaration=JSON.parse(declarationBytes),{start,end,lastDate}=validateWindowDeclaration(declaration);
    const predeclaration=authenticateDeclaration(declaration,start,snapshot);
    const endpoint=`repos/${repository}/actions/workflows/daemon-reliability.yml/runs?created=${declaration.window_start}..${lastDate}`;
    const before=pages(snapshot,"census-before",endpoint,"workflow_runs");
    const archive=readRetainedArchive(archiveRoot,{terminalEvidence:true});
    const scoped=archive.records.filter(r=>Date.parse(r.created_at)>=start && Date.parse(r.created_at)<end);
    const sourceTrees=authenticateRetainedArchive(archiveRoot,scoped,snapshot);
    const after=pages(snapshot,"census-after",endpoint,"workflow_runs");
    const ordered=rows=>[...rows].sort((a,b)=>a.id-b.id);
    if(!isDeepStrictEqual(ordered(before),ordered(after))) throw Error("complete census changed during collection");
    const ledger=buildWindowLedger(declaration,before,archive.records,new Date(Math.min(...dates)).toISOString());
    ledger.collector=collectorIdentity();ledger.source_trees=sourceTrees;ledger.declaration_file_sha256=hash(declarationBytes);ledger.separately_retained_collections=archive.separately_retained_collections;
    ledger.predeclaration_verified=true;ledger.predeclaration=predeclaration;
    if(!ledger.collector.source_clean) {ledger.window_qualified=false;ledger.source_limitation="collector checkout is not clean committed source";}
    writeFileSync(join(output,"ledger.json"),json(ledger),{flag:"wx"});
    const packet=`# Native daemon 30-day engineering ledger\n\nWindow: ${ledger.window_start} to ${ledger.window_end_exclusive} (exclusive). GitHub observation: ${ledger.server_observed_at}.\n\nFirst attempts: ${ledger.qualified_os_observations}/${ledger.expected_os_observations} OS opportunities (${ledger.first_attempt_percent}%). Qualified journeys: ${ledger.qualified_journeys}/${ledger.expected_journeys}. Engineering window qualified: ${ledger.window_qualified}.\n\n| UTC day | OS | State | Run | Series | Creation delay seconds |\n| --- | --- | --- | --- | --- | --- |\n`+ledger.rows.map(r=>`| ${r.date} | ${r.os} | ${r.state} | ${r.run_id ?? "missing"} | ${r.series ?? "unaccepted"} | ${r.arrival_delay_seconds ?? "unknown"} |`).join("\n")+"\n\nOriginal HTTP responses, archive reauthentication and exact source hashes are retained. Terminal counts cover retained census captures only. Declaration acceptance, human coherence/freeze, deployment and release are not inferred.\n";
    writeFileSync(join(output,"packet.md"),packet,{flag:"wx"});writeArtifactIndex(output);
    return {output,ledger};
  } catch(error) {
    writeFileSync(join(output,"failure.json"),json({qualified:false,error:error.message}),{flag:"wx"});writeArtifactIndex(output);throw error;
  }
}

if(process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if(process.argv.length !== 5) {console.error("usage: node scripts/qualify-daemon-window.mjs DECLARATION_JSON ARCHIVE_ROOT NEW_OUTPUT_DIRECTORY");process.exitCode=2;}
  else try {const {ledger}=collectWindow(...process.argv.slice(2));console.log(JSON.stringify({qualified:ledger.window_qualified,first_attempt_os_observations:ledger.qualified_os_observations,expected:90}));if(!ledger.window_qualified) process.exitCode=1;}
  catch(error) {console.error(error.message);process.exitCode=1;}
}
