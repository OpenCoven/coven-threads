import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {existsSync,mkdtempSync,mkdirSync,readFileSync,readdirSync,writeFileSync,rmSync,symlinkSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import test from 'node:test';
import {writeArtifactIndex} from '../lib/daemon-reliability-evidence.mjs';

async function implementation(){const p=new URL('../retain-nextest-evidence.mjs',import.meta.url);assert(existsSync(p),'separate immutable core collector is missing');return import(p);}
const hash=b=>createHash('sha256').update(b).digest('hex');
const blobHash=b=>createHash('sha1').update(`blob ${b.length}\0`).update(b).digest('hex');
function fixture(t){
 const root=mkdtempSync(join(tmpdir(),'threads-core-retention-'));t.after(()=>rmSync(root,{recursive:true,force:true}));
 const run={id:42,run_attempt:1,head_sha:'a'.repeat(40),status:'completed',conclusion:'success',event:'push',path:'.github/workflows/ci.yml',created_at:'2026-11-08T07:00:00Z',run_started_at:'2026-11-08T07:01:00Z',repository:{full_name:'OpenCoven/coven-threads'}};
 const job={id:100,run_id:42,run_attempt:1,head_sha:run.head_sha,name:'Nextest telemetry (JUnit, fail on flaky)',status:'completed',conclusion:'success',started_at:'2026-11-08T07:20:00Z',completed_at:'2026-11-08T07:30:00Z',labels:['ubuntu-latest'],steps:[{name:'Run tests with observable retry semantics',conclusion:'success',started_at:'2026-11-08T07:26:00Z',completed_at:'2026-11-08T07:27:00Z'},{name:'Upload Nextest JUnit',conclusion:'success',started_at:'2026-11-08T07:27:00Z',completed_at:'2026-11-08T07:28:00Z'}]};
 const uuid='12345678-1234-1234-1234-123456789abc';
 const xml=`<testsuites uuid="${uuid}" timestamp="2026-11-08T07:26:30Z" tests="1" failures="0" errors="0"><testsuite name="core" tests="1" failures="0" errors="0"><testcase classname="core" name="one"/></testsuite></testsuites>`;
 const producer=join(root,'producer');mkdirSync(producer);writeFileSync(join(producer,'junit.xml'),xml);const zip=join(root,'original.zip');execFileSync('python3',['-I','-c','import zipfile,sys; z=zipfile.ZipFile(sys.argv[2],"w"); z.write(sys.argv[1],"junit.xml"); z.close()',join(producer,'junit.xml'),zip]);
 const bytes=readFileSync(zip);
 const artifact={id:200,name:'nextest-junit',expired:false,size_in_bytes:bytes.length,digest:'sha256:'+hash(bytes),created_at:'2026-11-08T07:27:30Z',updated_at:'2026-11-08T07:27:30Z',workflow_run:{id:42,head_sha:run.head_sha}};
 const config={'Cargo.lock':'version = 4\n','Cargo.toml':'[workspace]\n','rust-toolchain.toml':'[toolchain]\nchannel = "1.88.0"\n','.config/nextest.toml':'[profile.ci]\nretries = 1\nflaky-result = "fail"\n[profile.ci.junit]\npath = "junit.xml"\nflaky-fail-status = "failure"\n','.github/workflows/ci.yml':'name: CI\n','.node-version':'24.22.0\n'};
 const blobs=new Map(Object.values(config).map(text=>{const b=Buffer.from(text);return [blobHash(b),{encoding:'base64',sha:blobHash(b),size:b.length,content:b.toString('base64')}];}));
 const tree={sha:'b'.repeat(40),truncated:false,tree:[...Object.entries(config).map(([path,text])=>({path,type:'blob',mode:'100644',sha:blobHash(Buffer.from(text)),size:Buffer.byteLength(text)})),{path:'src/lib.rs',type:'blob',mode:'100644',sha:'c'.repeat(40),size:20}]};
 const log=`[command]/usr/bin/git log -1 --format=%H\n${run.head_sha}\n##[group]Runner Image\nImage: ubuntu-24.04\nVersion: 20261101.360.1\nrustc 1.88.0 (synthetic tooling fixture)\nhost: x86_64-unknown-linux-gnu\ncargo-nextest 0.9.143 (synthetic tooling fixture)\n##[group]Run cargo nextest run --locked --workspace --profile ci\ncargo nextest run --locked --workspace --profile ci\nNextest run ID ${uuid} with nextest profile: ci\nSummary [ 1s ] 1 tests run: 1 passed, 0 skipped\n`;
 const f={root,archive:join(root,'archive'),run,jobs:[job],artifacts:[artifact],zip:bytes,log,tree,blobs,afterChanged:false,reads:0};
 f.read=ep=>{let v;
  if(ep.includes('/jobs?'))v={total_count:f.jobs.length,jobs:f.jobs};
  else if(ep.includes('/artifacts?'))v={total_count:f.artifacts.length,artifacts:f.artifacts};
  else if(ep.endsWith('/zip'))return f.zip;
  else if(ep.endsWith('/logs'))return Buffer.from(f.log);
  else if(ep.includes('/git/commits/'))v={sha:run.head_sha,tree:{sha:tree.sha},parents:[]};
  else if(ep.includes('/git/trees/'))v=f.tree;
  else if(ep.includes('/git/blobs/'))v=f.blobs.get(ep.split('/').at(-1));
  else if(ep.endsWith('/attempts/1')){f.reads++;v=f.afterChanged&&f.reads>1?{...run,conclusion:'failure'}:run;}
  else throw Error('unexpected endpoint '+ep);
  return Buffer.from(JSON.stringify(v));
 };
 return f;
}
test('collects original bytes and exact source once, then replays idempotently',async t=>{
 const{collectNextestAttempt,summarizeNextestArchive}=await implementation();const f=fixture(t);
 assert.equal(collectNextestAttempt(42,1,f.archive,f.read).state,'created');
 const root=join(f.archive,'42','1');assert.deepEqual(readFileSync(join(root,'original.zip')),f.zip);
 const record=JSON.parse(readFileSync(join(root,'record.json')));assert.equal(record.first_attempt_passed,true);assert.equal(record.source.config.length,6);
 assert.equal(collectNextestAttempt(42,1,f.archive,f.read).state,'identical');assert.equal(summarizeNextestArchive(f.archive).first_attempt_passes,1);
});
test('changed run snapshots retain a failure outside canonical identity',async t=>{
 const{collectNextestAttempt}=await implementation();const f=fixture(t);f.afterChanged=true;
 assert.throws(()=>collectNextestAttempt(42,1,f.archive,f.read));assert(!existsSync(join(f.archive,'42','1')));assert(readdirSync(f.archive).some(p=>p.startsWith('.collect-nextest-')));
});
test('corrupt ZIP is retained as negative evidence and cannot become a pass',async t=>{
 const{collectNextestAttempt,summarizeNextestArchive}=await implementation();const f=fixture(t);f.zip=Buffer.from('not a zip');
 const r=collectNextestAttempt(42,1,f.archive,f.read);assert.equal(r.record.first_attempt_passed,false);assert.equal(r.record.state,'artifact_validation_failed');assert.equal(summarizeNextestArchive(f.archive).first_attempt_passes,0);
});
test('expired and missing artifacts and setup failure retain explicit gaps',async t=>{
 const{collectNextestAttempt,summarizeNextestArchive}=await implementation();const f=fixture(t);f.artifacts[0].expired=true;
 collectNextestAttempt(42,1,f.archive,f.read);const s=summarizeNextestArchive(f.archive);assert.equal(s.gaps[0].state,'artifact_expired');assert.equal(s.first_attempt_passes,0);
});
test('coherent record and index tampering cannot manufacture first-attempt success',async t=>{
 const{collectNextestAttempt,summarizeNextestArchive}=await implementation();const f=fixture(t);f.artifacts=[];
 collectNextestAttempt(42,1,f.archive,f.read);const root=join(f.archive,'42','1');const p=join(root,'record.json');const record=JSON.parse(readFileSync(p));record.first_attempt_passed=true;record.execution_passed=true;record.state='qualified';writeFileSync(p,JSON.stringify(record));rmSync(join(root,'artifact-index.json'));writeArtifactIndex(root);
 assert.throws(()=>summarizeNextestArchive(f.archive));
});
test('conflicting immutable identity preserves the original canonical bytes',async t=>{
 const{collectNextestAttempt,summarizeNextestArchive}=await implementation();const f=fixture(t);collectNextestAttempt(42,1,f.archive,f.read);const p=join(f.archive,'42/1/record.json'),before=readFileSync(p);f.log+='retained differing bytes\n';assert.throws(()=>collectNextestAttempt(42,1,f.archive,f.read));assert.deepEqual(readFileSync(p),before);assert.equal(summarizeNextestArchive(f.archive).first_attempt_passes,1);assert.equal(summarizeNextestArchive(f.archive).incomplete_collections,1);
});
test('a run-directory symlink cannot publish an attempt outside the dedicated archive',async t=>{
 const{collectNextestAttempt}=await implementation();const f=fixture(t);f.artifacts=[];const outside=join(f.root,'outside');mkdirSync(outside);mkdirSync(f.archive);writeFileSync(join(f.archive,'profile.json'),JSON.stringify({schema_version:'opencoven.threads-nextest-archive/v1',profile:'core-nextest-ci',repository:'OpenCoven/coven-threads'}));symlinkSync(outside,join(f.archive,'42'));
 assert.throws(()=>collectNextestAttempt(42,1,f.archive,f.read));assert.deepEqual(readdirSync(outside),[]);
});
test('core ingestion refuses a nonempty unmarked or native archive before writing',async t=>{
 const{collectNextestAttempt}=await implementation();const f=fixture(t);mkdirSync(f.archive);writeFileSync(join(f.archive,'native-record.json'),'original');const before=readdirSync(f.archive);assert.throws(()=>collectNextestAttempt(42,1,f.archive,f.read));assert.deepEqual(readdirSync(f.archive),before);
});
test('failed artifact transport is replayable as a retained negative observation',async t=>{
 const{collectNextestAttempt,summarizeNextestArchive}=await implementation();const f=fixture(t);const read=f.read;f.read=ep=>{if(ep.endsWith('/zip'))throw Error('network transport unavailable');return read(ep);};
 const r=collectNextestAttempt(42,1,f.archive,f.read);assert.equal(r.record.first_attempt_passed,false);assert.equal(summarizeNextestArchive(f.archive).gaps[0].state,'artifact_validation_failed');
});
test('source blob contradiction cannot qualify and survives offline replay',async t=>{
 const{collectNextestAttempt,summarizeNextestArchive}=await implementation();const f=fixture(t);for(const blob of f.blobs.values())blob.content=Buffer.from('wrong bytes').toString('base64');
 const r=collectNextestAttempt(42,1,f.archive,f.read);assert.equal(r.record.first_attempt_passed,false);assert.equal(r.record.report.counts.tests,1);const s=summarizeNextestArchive(f.archive);assert.equal(s.first_attempt_passes,0);assert.equal(s.gaps[0].state,'artifact_validation_failed');
});
