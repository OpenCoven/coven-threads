#!/usr/bin/env node
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {existsSync,lstatSync,mkdirSync,mkdtempSync,readFileSync,readdirSync,realpathSync,rmSync,writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {classifyNextestMetadata,nextestHash,parseNextestJunit,qualifyNextest,summarizeNextest} from './lib/nextest-retention.mjs';
import {extractArtifact,publishArchive,verifyArtifactIndex} from './lib/daemon-artifact-archive.mjs';
import {writeArtifactIndex} from './lib/daemon-reliability-evidence.mjs';

const repository='OpenCoven/coven-threads';
const profile={schema_version:'opencoven.threads-nextest-archive/v1',profile:'core-nextest-ci',repository};
const json=value=>JSON.stringify(value,null,2)+'\n';
const safeId=n=>Number.isSafeInteger(n)&&n>0;
const sha=s=>typeof s==='string'&&/^[a-f0-9]{40}$/.test(s);
const gitBlobHash=b=>createHash('sha1').update(`blob ${b.length}\0`).update(b).digest('hex');
const configPath=p=>/(^|\/)(Cargo\.(toml|lock)|rust-toolchain\.toml|config(?:\.toml)?)$/.test(p)||['.config/nextest.toml','.github/workflows/ci.yml','.node-version'].includes(p);
function regular(path){if(!lstatSync(path).isFile())throw Error('regular retained file required');return readFileSync(path);}
function githubRead(endpoint){const r=spawnSync('gh',['api',endpoint],{maxBuffer:256*1024*1024});if(r.error||r.status!==0)throw Error('authenticated GitHub REST read failed');return r.stdout;}
function openArchive(path,create=false){
 const root=resolve(path);if(create)mkdirSync(root,{recursive:true});
 if(!lstatSync(root).isDirectory())throw Error('dedicated regular core archive required');
 const marker=join(root,'profile.json');
 if(!existsSync(marker)){
  if(!create||readdirSync(root).length)throw Error('refusing nonempty unmarked or native archive');
  try{writeFileSync(marker,json(profile),{flag:'wx'});}catch(e){if(e.code!=='EEXIST')throw e;}
 }
 assert.deepEqual(JSON.parse(regular(marker)),profile,'foreign archive profile');
 return realpathSync(root);
}
function sourceReceipt(log,read){
 const clean=log.toString().replace(/\x1b\[[0-?]*[ -/]*[@-~]/g,'').replace(/^\d{4}-\d\d-\d\dT\S+Z /gm,'');
 const commits=[...clean.matchAll(/\[command\][^\n]*git log -1 --format=%H\r?\n([a-f0-9]{40})(?:\r?\n|$)/g)].map(m=>m[1]);
 if(commits.length!==1)throw Error('missing or ambiguous actual checkout');
 const prefix=`repos/${repository}/git`,commit=JSON.parse(read(`${prefix}/commits/${commits[0]}`));
 if(commit.sha!==commits[0]||!sha(commit.tree?.sha)||!Array.isArray(commit.parents))throw Error('source commit metadata mismatch');
 const tree=JSON.parse(read(`${prefix}/trees/${commit.tree.sha}?recursive=1`));
 if(tree.sha!==commit.tree.sha||tree.truncated!==false||!Array.isArray(tree.tree)||new Set(tree.tree.map(e=>e.path)).size!==tree.tree.length)throw Error('incomplete or ambiguous source tree');
 const files=tree.tree.filter(e=>e.type==='blob');
 const sources=files.filter(e=>e.path.endsWith('.rs')).sort((a,b)=>a.path.localeCompare(b.path,'en'));
 const configs=files.filter(e=>configPath(e.path)).sort((a,b)=>a.path.localeCompare(b.path,'en'));
 for(const p of ['Cargo.lock','Cargo.toml','rust-toolchain.toml','.config/nextest.toml','.github/workflows/ci.yml','.node-version'])if(!configs.some(e=>e.path===p))throw Error('required source configuration missing');
 if(!sources.length)throw Error('no tracked Rust source');
 const config=[];
 for(const e of [...sources,...configs])if(!sha(e.sha)||!['100644','100755'].includes(e.mode)||e.path.startsWith('/')||e.path.split('/').some(p=>!p||p==='.'||p==='..'))throw Error('unsafe source/configuration entry');
 for(const e of configs){
  const blob=JSON.parse(read(`${prefix}/blobs/${e.sha}`));
  if(blob.sha!==e.sha||blob.encoding!=='base64'||typeof blob.content!=='string')throw Error('source blob identity missing');
  const bytes=Buffer.from(blob.content,'base64');
  if(blob.size!==bytes.length||e.size!==bytes.length||gitBlobHash(bytes)!==e.sha)throw Error('source configuration blob digest mismatch');
  config.push({path:e.path,git_sha:e.sha,sha256:nextestHash(bytes),bytes:bytes.length});
 }
 return {commit:commit.sha,tree:commit.tree.sha,parents:commit.parents.map(p=>p.sha),config,sourcePaths:sources.map(e=>e.path),sourceGitIdentities:sources.map(e=>({path:e.path,git_sha:e.sha}))};
}
function buildAttempt(runId,attempt,read,directory){
 const prefix=`repos/${repository}/actions/runs/${runId}`;
 const get=ep=>JSON.parse(read(ep));
 const pages=(ep,key)=>{const rows=[];let total;
  for(let page=1;page<=100;page++){
   const data=get(`${ep}?per_page=100&page=${page}`);
   if(!Number.isSafeInteger(data.total_count)||data.total_count<0||!Array.isArray(data[key]))throw Error('invalid paginated metadata');
   total??=data.total_count;if(total!==data.total_count)throw Error('metadata changed during pagination');rows.push(...data[key]);
   if(rows.length===total)return rows;if(rows.length>total||!data[key].length)throw Error('incomplete metadata pages');
  }throw Error('metadata exceeds page bound');
 };
 const run=get(`${prefix}/attempts/${attempt}`);if(run.id!==runId||run.run_attempt!==attempt)throw Error('explicit requested attempt differs');
 const jobs=pages(`${prefix}/attempts/${attempt}/jobs`,'jobs'),artifacts=pages(`${prefix}/artifacts`,'artifacts');
 let record=classifyNextestMetadata({run,jobs,artifacts});
 if(record.artifact&&!record.artifact.expired){
  try{
   const zip=read(`repos/${repository}/actions/artifacts/${record.artifact.id}/zip`);
   writeFileSync(join(directory,'validation.zip'),zip,{flag:'wx'});
   try{extractArtifact(join(directory,'validation.zip'),record.artifact.digest?.slice(7),join(directory,'evidence'));}finally{rmSync(join(directory,'validation.zip'));}
   if(JSON.stringify(readdirSync(join(directory,'evidence')).sort())!==JSON.stringify(['junit.xml']))throw Error('unexpected core artifact file inventory');
   if(zip.length!==record.artifact.size_in_bytes)throw Error('artifact byte length mismatch');
   const xml=regular(join(directory,'evidence','junit.xml'));
   record={...record,report:parseNextestJunit(xml)};
   const log=read(`repos/${repository}/actions/jobs/${record.job.id}/logs`);
   const source=sourceReceipt(log,read);
   record=qualifyNextest(record,xml,log,source);
  }catch(error){record={...record,state:'artifact_validation_failed',qualification_error:error.message};}
 }
 const jobsAfter=pages(`${prefix}/attempts/${attempt}/jobs`,'jobs'),artifactsAfter=pages(`${prefix}/artifacts`,'artifacts'),runAfter=get(`${prefix}/attempts/${attempt}`);
 const ordered=a=>[...a].sort((x,y)=>x.id-y.id);
 assert.deepEqual(runAfter,run,'run changed during collection');assert.deepEqual(ordered(jobsAfter),ordered(jobs),'jobs changed during collection');assert.deepEqual(ordered(artifactsAfter),ordered(artifacts),'artifacts changed during collection');
 return record;
}
function packet(record){
 const c=record.report?.counts;
 return `# Retained core Nextest attempt ${record.run_id}/${record.attempt}\n\n`+
 `Core CI source ${record.head_sha}; checkout ${record.source?.commit??'unproven'}; event ${record.event}; state ${record.state}.\n\n`+
 `Workflow first-attempt pass: ${record.first_attempt_passed}. Tests: ${c?.tests??'unknown'}; first-execution passes: ${c?.firstAttemptPassed??'unknown'}; flaky tests: ${c?.flaky??'unknown'}; retries: ${c?.retries??'unknown'}.\n\n`+
 'Original authenticated metadata, available ZIP/log/source bytes and failures are retained. This is core telemetry, not native daemon qualification, a causal flake diagnosis or a 30-day retention claim.\n';
}
export function collectNextestAttempt(runId,attempt,archiveRoot,read=githubRead){
 if(!safeId(runId)||!safeId(attempt))throw Error('positive explicit run and attempt required');
 const root=openArchive(archiveRoot,true),parent=join(root,String(runId));
 try{mkdirSync(parent);}catch(e){if(e.code!=='EEXIST')throw e;}
 if(!lstatSync(parent).isDirectory())throw Error('run destination must be a regular directory');
 const destination=join(parent,String(attempt)),existing=lstatSync(destination,{throwIfNoEntry:false});
 if(existing&&!existing.isDirectory())throw Error('attempt destination must be a regular directory');
 const staging=mkdtempSync(join(root,`.collect-nextest-${runId}-${attempt}-`));mkdirSync(join(staging,'metadata'));
 const snapshots=[];
 const retainedRead=endpoint=>{
  let bytes;
  try{bytes=Buffer.from(read(endpoint));}catch{snapshots.push({endpoint,error:'authenticated REST read failed'});throw Error('authenticated REST read failed');}
  const path=endpoint.endsWith('/zip')?'original.zip':endpoint.endsWith('/logs')?'job.log':`metadata/${snapshots.length}.json`;
  writeFileSync(join(staging,path),bytes,{flag:'wx'});snapshots.push({endpoint,path,bytes:bytes.length,sha256:nextestHash(bytes)});return bytes;
 };
 try{
  const record=buildAttempt(runId,attempt,retainedRead,staging);
  writeFileSync(join(staging,'record.json'),json(record),{flag:'wx'});writeFileSync(join(staging,'snapshots.json'),json(snapshots),{flag:'wx'});writeFileSync(join(staging,'packet.md'),packet(record),{flag:'wx'});
  writeArtifactIndex(staging);return {...publishArchive(staging,destination),record};
 }catch(error){
  writeFileSync(join(staging,'collection-failure.json'),json({run_id:runId,attempt,error:error.message}),{flag:'wx'});
  if(!existsSync(join(staging,'snapshots.json')))writeFileSync(join(staging,'snapshots.json'),json(snapshots),{flag:'wx'});
  rmSync(join(staging,'artifact-index.json'),{force:true});writeArtifactIndex(staging);
  throw new Error('core collection refused; original failure snapshot retained',{cause:error});
 }
}
function replayAttempt(directory,runId,attempt){
 verifyArtifactIndex(directory);
 const snapshots=JSON.parse(regular(join(directory,'snapshots.json')));let cursor=0;
 if(!Array.isArray(snapshots)||!snapshots.length)throw Error('missing replay snapshot inventory');
 const read=endpoint=>{
  const row=snapshots[cursor++];if(row?.endpoint!==endpoint)throw Error('replay request identity mismatch');
  if(row.error){if(row.error!=='authenticated REST read failed'||row.path)throw Error('invalid retained transport failure');throw Error(row.error);}
  if(typeof row.path!=='string'||!/^(?:metadata\/[0-9]+\.json|original\.zip|job\.log)$/.test(row.path))throw Error('replay request path mismatch');
  const bytes=regular(join(directory,row.path));if(bytes.length!==row.bytes||nextestHash(bytes)!==row.sha256)throw Error('replay snapshot digest mismatch');return bytes;
 };
 const temp=mkdtempSync(join(tmpdir(),'threads-nextest-replay-'));
 try{
  const replayed=buildAttempt(runId,attempt,read,temp);if(cursor!==snapshots.length)throw Error('unused or missing replay snapshots');
  const saved=JSON.parse(regular(join(directory,'record.json')));assert.deepEqual(saved,replayed,'saved claims differ from replayed original evidence');return replayed;
 }finally{rmSync(temp,{recursive:true,force:true});}
}
export function summarizeNextestArchive(archiveRoot){
 const root=openArchive(archiveRoot),records=[];let incomplete=0;
 for(const run of readdirSync(root).sort()){
  if(run==='profile.json')continue;
  if(run.startsWith('.collect-nextest-')){verifyArtifactIndex(join(root,run));incomplete++;continue;}
  if(!/^[1-9][0-9]*$/.test(run)||!lstatSync(join(root,run)).isDirectory())throw Error('foreign core archive entry');
  for(const attempt of readdirSync(join(root,run)).sort()){
   if(!/^[1-9][0-9]*$/.test(attempt)||!safeId(Number(run))||!safeId(Number(attempt))||!lstatSync(join(root,run,attempt)).isDirectory())throw Error('foreign core attempt identity');
   records.push(replayAttempt(join(root,run,attempt),Number(run),Number(attempt)));
  }
 }
 return {...summarizeNextest(records),incomplete_collections:incomplete};
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 try{
  if(process.argv.length===4&&process.argv[2]==='--summary')console.log(json(summarizeNextestArchive(process.argv[3])));
  else if(process.argv.length===5){const r=collectNextestAttempt(Number(process.argv[2]),Number(process.argv[3]),process.argv[4]);console.log(json({state:r.state,run_id:r.record.run_id,attempt:r.record.attempt,first_attempt_passed:r.record.first_attempt_passed,observation:r.record.state}));}
  else throw Error('usage: retain-nextest-evidence.mjs RUN_ID ATTEMPT DEDICATED_CORE_ARCHIVE | --summary DEDICATED_CORE_ARCHIVE');
 }catch(error){console.error(error.message);process.exitCode=1;}
}
