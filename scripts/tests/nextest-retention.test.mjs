import assert from 'node:assert/strict';
import {existsSync} from 'node:fs';
import test from 'node:test';

async function implementation() {
  const url=new URL('../lib/nextest-retention.mjs',import.meta.url);
  assert(existsSync(url),'separate core Nextest retention profile is missing');
  return import(url);
}
const uuid='12345678-1234-1234-1234-123456789abc';
const at='2026-11-08T07:26:51.057+00:00';
const report=body=>`<testsuites name="nextest-run" uuid="${uuid}" timestamp="${at}" tests="1" failures="0" errors="0"><testsuite name="core" tests="1" failures="0" errors="0">${body}</testsuite></testsuites>`;
const pass=report('<testcase name="test_one" classname="core"/>');

test('a real report derives unique test and first-execution counts',async()=>{
  const {parseNextestJunit}=await implementation();const r=parseNextestJunit(Buffer.from(pass));
  assert.deepEqual(r.counts,{tests:1,passed:1,failures:0,errors:0,skipped:0,flaky:0,retried:0,retries:0,firstAttemptPassed:1});
  assert.equal(r.uuid,uuid);assert.equal(r.cases[0].name,'test_one');
});
test('flaky-fail retains both terminal failure and recovery markers without doubling tests',async()=>{
  const {parseNextestJunit}=await implementation();
  const xml=report('<testcase name="one" classname="core"><failure>flaky policy</failure><flakyFailure><system-err>original fail</system-err></flakyFailure></testcase>').replaceAll('failures="0"','failures="1"');
  const r=parseNextestJunit(Buffer.from(xml));
  assert.deepEqual(r.counts,{tests:1,passed:0,failures:1,errors:0,skipped:0,flaky:1,retried:1,retries:1,firstAttemptPassed:0});
  assert.equal(r.cases[0].flakyFailures,1);
});
test('terminal retry failures and skipped/error rows stay distinct',async()=>{
  const {parseNextestJunit}=await implementation();
  const xml=`<testsuites uuid="${uuid}" timestamp="${at}" tests="4" failures="1" errors="1" skipped="1"><testsuite name="core" tests="4" failures="1" errors="1" skipped="1"><testcase name="p" classname="core"/><testcase name="f" classname="core"><failure/><rerunFailure/><rerunError/></testcase><testcase name="e" classname="core"><error/></testcase><testcase name="s" classname="core"><skipped/></testcase></testsuite></testsuites>`;
  assert.deepEqual(parseNextestJunit(Buffer.from(xml)).counts,{tests:4,passed:1,failures:1,errors:1,skipped:1,flaky:0,retried:1,retries:2,firstAttemptPassed:1});
});
for(const [label,change]of[
 ['malformed',()=>'<testsuites>'],['empty',()=>report('').replaceAll('tests="1"','tests="0"')],
 ['false count',x=>x.replaceAll('tests="1"','tests="2"')],['duplicate identity',x=>x.replace('</testsuite>','<testcase name="test_one" classname="core"/></testsuite>').replaceAll('tests="1"','tests="2"')],
 ['nested report',x=>x.replace('<testcase','<testsuite><testcase').replace('/></testsuite>','/></testsuite></testsuite>')],
 ['unknown outcome',x=>x.replace('/>','><surprise/></testcase>')],['mixed terminal',()=>report('<testcase name="a" classname="core"><skipped/><failure/></testcase>')],
 ['entity',x=>'<!DOCTYPE testsuites [<!ENTITY example "value">]>'+x],['missing UUID',x=>x.replace(`uuid="${uuid}"`,'')],['invalid timestamp',x=>x.replace(at,'tomorrow')],
 ['unknown retry',()=>report('<testcase name="a" classname="core"><rerunFailure/></testcase>')],
])test(`JUnit refuses ${label}`,async()=>{const{parseNextestJunit}=await implementation();assert.throws(()=>parseNextestJunit(Buffer.from(change(pass))));});

export function metadataFixture() {
 const run={id:42,run_attempt:1,head_sha:'a'.repeat(40),status:'completed',conclusion:'success',event:'push',path:'.github/workflows/ci.yml',created_at:'2026-11-08T07:00:00Z',run_started_at:'2026-11-08T07:01:00Z',repository:{full_name:'OpenCoven/coven-threads'}};
 const job={id:100,run_id:42,run_attempt:1,head_sha:run.head_sha,name:'Nextest telemetry (JUnit, fail on flaky)',status:'completed',conclusion:'success',started_at:'2026-11-08T07:20:00Z',completed_at:'2026-11-08T07:30:00Z',labels:['ubuntu-latest'],steps:[{name:'Run tests with observable retry semantics',conclusion:'success',started_at:'2026-11-08T07:26:00Z',completed_at:'2026-11-08T07:27:00Z'},{name:'Upload Nextest JUnit',conclusion:'success',started_at:'2026-11-08T07:27:00Z',completed_at:'2026-11-08T07:28:00Z'}]};
 const artifact={id:200,name:'nextest-junit',expired:false,digest:'sha256:'+'c'.repeat(64),created_at:'2026-11-08T07:27:30Z',updated_at:'2026-11-08T07:27:30Z',workflow_run:{id:42,head_sha:run.head_sha}};
 return {run,jobs:[job],artifacts:[artifact]};
}
test('generic artifact names require exact attempt and upload-interval binding',async()=>{
 const{classifyNextestMetadata}=await implementation();const f=metadataFixture();assert.equal(classifyNextestMetadata(f).artifact.id,200);
 f.artifacts[0].created_at='2026-11-08T06:00:00Z';assert.throws(()=>classifyNextestMetadata(f));
});
for(const [label,change]of[
 ['foreign workflow',f=>f.run.path='.github/workflows/daemon-reliability.yml'],['wrong attempt',f=>f.jobs[0].run_attempt=2],['duplicate job',f=>f.jobs.push({...f.jobs[0],id:101})],['duplicate artifact',f=>f.artifacts.push({...f.artifacts[0],id:201})],['foreign source',f=>f.artifacts[0].workflow_run.head_sha='b'.repeat(40)],['running job',f=>f.jobs[0].status='in_progress']
])test(`metadata refuses ${label}`,async()=>{const{classifyNextestMetadata}=await implementation();const f=metadataFixture();change(f);assert.throws(()=>classifyNextestMetadata(f));});
test('missing expired cancelled and setup-failed observations remain negative',async()=>{
 const{classifyNextestMetadata}=await implementation();
 for(const [kind,change]of[['artifact_missing',f=>f.artifacts=[]],['artifact_expired',f=>f.artifacts[0].expired=true],['job_missing',f=>{f.jobs=[];f.artifacts=[];}],['job_cancelled',f=>{f.jobs[0].conclusion='cancelled';f.artifacts=[];}],['job_failure',f=>{f.jobs[0].conclusion='failure';f.artifacts=[];}]]){
  const f=metadataFixture();change(f);const r=classifyNextestMetadata(f);assert.equal(r.state,kind);assert.equal(r.first_attempt_passed,false);
 }
});
function qualificationFixture() {
 const source={commit:'a'.repeat(40),tree:'d'.repeat(40),parents:[],config:[{path:'Cargo.lock',sha256:'e'.repeat(64)}],sourcePaths:['src/lib.rs']};
 const log=`[command]/usr/bin/git log -1 --format=%H\n${source.commit}\n##[group]Runner Image\nImage: ubuntu-24.04\nVersion: 20261101.360.1\nrustc 1.88.0 (synthetic tooling fixture)\nhost: x86_64-unknown-linux-gnu\ncargo-nextest 0.9.143 (synthetic tooling fixture)\ncargo nextest run --locked --workspace --profile ci\nNextest run ID ${uuid} with nextest profile: ci\nSummary [ 1s ] 1 tests run: 1 passed, 0 skipped\n`;
 return {source,log,xml:Buffer.from(pass),metadata:metadataFixture()};
}
test('qualification binds actual checkout, JUnit UUID, timestamp and hosted tool scope',async()=>{
 const{classifyNextestMetadata,qualifyNextest}=await implementation();const f=qualificationFixture();
 const r=qualifyNextest(classifyNextestMetadata(f.metadata),f.xml,f.log,f.source);
 assert.equal(r.first_attempt_passed,true);assert.equal(r.report.counts.tests,1);assert.equal(r.scope.cargoVersion,null);assert.match(r.scope_sha256,/^[a-f0-9]{64}$/);
});
for(const[label,change]of[
 ['foreign UUID',f=>f.log=f.log.replace(uuid,'87654321-4321-4321-4321-cba987654321')],
 ['foreign commit',f=>f.source.commit='b'.repeat(40)],['ambiguous checkout',f=>f.log+=`[command]/usr/bin/git log -1 --format=%H\n${f.source.commit}\n`],
 ['unrelated PR tree',f=>{f.source.commit='b'.repeat(40);f.log=f.log.replace('a'.repeat(40),'b'.repeat(40));}],
 ['outside step',f=>f.xml=Buffer.from(pass.replace(at,'2026-11-07T00:00:00Z'))],['missing tools',f=>f.log=f.log.replace('cargo-nextest 0.9.143','missing 0.9.143')],
 ['missing source inventory',f=>f.source.sourcePaths=[]],['contradictory summary',f=>f.log=f.log.replace('1 tests run: 1 passed','2 tests run: 2 passed')]
])test(`qualification refuses ${label}`,async()=>{const{classifyNextestMetadata,qualifyNextest}=await implementation();const f=qualificationFixture();change(f);assert.throws(()=>qualifyNextest(classifyNextestMetadata(f.metadata),f.xml,f.log,f.source));});
test('flaky-pass JUnit remains observed but never a first-execution pass',async()=>{
 const{classifyNextestMetadata,qualifyNextest}=await implementation();const f=qualificationFixture();f.xml=Buffer.from(pass.replace('/>','><flakyFailure/></testcase>'));
 const r=qualifyNextest(classifyNextestMetadata(f.metadata),f.xml,f.log,f.source);assert.equal(r.first_attempt_passed,false);assert.equal(r.report.counts.flaky,1);
});
test('summary keeps original failures later recovery and source scope separate',async()=>{
 const{classifyNextestMetadata,summarizeNextest}=await implementation();const f=metadataFixture();f.artifacts=[];f.jobs[0].conclusion='failure';const first=classifyNextestMetadata(f);
 const later={...first,attempt:2,state:'qualified',execution_passed:true,first_attempt_passed:false,scope_sha256:'b'.repeat(64)};
 const r=summarizeNextest([later,first]);assert.equal(r.observed_runs,1);assert.equal(r.first_attempts,1);assert.equal(r.first_attempt_passes,0);assert.equal(r.recovered_on_later_attempt,1);assert.equal(r.native_qualification,false);
 assert.throws(()=>summarizeNextest([first,first]));assert.deepEqual(r,summarizeNextest([first,later]));
});
