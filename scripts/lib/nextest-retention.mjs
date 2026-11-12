import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';

export const nextestHash=bytes=>createHash('sha256').update(bytes).digest('hex');
const refuse=message=>{throw new Error(message);};
const id=n=>Number.isSafeInteger(n)&&n>0;
const sha=s=>typeof s==='string'&&/^[a-f0-9]{40}$/.test(s);
const time=s=>typeof s==='string'&&Number.isFinite(Date.parse(s));
const inside=(s,start,end)=>time(s)&&time(start)&&time(end)&&Date.parse(s)>=Date.parse(start)&&Date.parse(s)<=Date.parse(end);
export const nextestJobName='Nextest telemetry (JUnit, fail on flaky)';
export function parseNextestJunit(bytes) {
  const r=spawnSync('python3',['-I',fileURLToPath(new URL('./nextest-junit.py',import.meta.url))],{input:bytes,maxBuffer:64*1024*1024});
  if(r.error||r.status!==0)refuse('Nextest JUnit refused');
  return JSON.parse(r.stdout);
}
export function classifyNextestMetadata({run,jobs,artifacts}) {
  if(!run||!id(run.id)||!id(run.run_attempt)||!sha(run.head_sha)||run.repository?.full_name!=='OpenCoven/coven-threads'||run.path!=='.github/workflows/ci.yml'||run.status!=='completed'||!['push','pull_request','workflow_dispatch','schedule','merge_group'].includes(run.event)||!time(run.created_at)||!Array.isArray(jobs)||!Array.isArray(artifacts))refuse('unsupported core CI run identity');
  if(new Set(jobs.map(j=>j.id)).size!==jobs.length||new Set(artifacts.map(a=>a.id)).size!==artifacts.length)refuse('duplicate metadata identity');
  const matches=jobs.filter(j=>j.name===nextestJobName);if(matches.length>1)refuse('ambiguous Nextest job');
  const job=matches[0]??null;
  if(job&&(!id(job.id)||job.run_id!==run.id||job.run_attempt!==run.run_attempt||job.head_sha!==run.head_sha||job.status!=='completed'||!['success','failure','cancelled','timed_out','skipped','neutral','action_required','stale'].includes(job.conclusion)))refuse('Nextest job identity differs from requested attempt');
  const candidates=artifacts.filter(a=>a.name==='nextest-junit');
  const upload=job?.steps?.filter(s=>s.name==='Upload Nextest JUnit')??[];
  if(upload.length>1)refuse('ambiguous upload step');
  let selected=[];
  for(const a of candidates) {
    if(!id(a.id)||a.workflow_run?.id!==run.id||a.workflow_run?.head_sha!==run.head_sha||typeof a.expired!=='boolean')refuse('foreign artifact identity');
    if(upload[0]&&inside(a.created_at,upload[0].started_at,upload[0].completed_at))selected.push(a);
  }
  if(selected.length>1||(!selected.length&&candidates.length))refuse('generic artifact cannot be bound to a unique attempt upload');
  const artifact=selected[0]??null;
  let state=!job?'job_missing':job.conclusion!=='success'?`job_${job.conclusion}`:!artifact?'artifact_missing':artifact.expired?'artifact_expired':'evidence_pending';
  if(artifact&&!artifact.expired&&!/^sha256:[a-f0-9]{64}$/.test(artifact.digest??''))state='artifact_digest_missing';
  return {schema_version:'opencoven.threads-nextest-retention/v1',run_id:run.id,attempt:run.run_attempt,head_sha:run.head_sha,event:run.event,created_at:run.created_at,started_at:run.run_started_at??null,job,artifact,state,execution_passed:false,first_attempt_passed:false,report:null,source:null,scope_sha256:null};
}
export function qualifyNextest(record,xml,rawLog,source) {
  const report=parseNextestJunit(xml);
  const log=rawLog.toString().replace(/\x1b\[[0-?]*[ -/]*[@-~]/g,'').replace(/^\d{4}-\d\d-\d\dT\S+Z /gm,'');
  const commits=[...log.matchAll(/\[command\][^\n]*git log -1 --format=%H\r?\n([a-f0-9]{40})(?:\r?\n|$)/g)].map(m=>m[1]);
  if(commits.length!==1||source?.commit!==commits[0]||!sha(source.tree)||!Array.isArray(source.config)||!source.config.length||!Array.isArray(source.sourcePaths)||!source.sourcePaths.length||new Set(source.sourcePaths).size!==source.sourcePaths.length)refuse('source checkout/configuration is missing or ambiguous');
  if(source.commit!==record.head_sha&&(record.event!=='pull_request'||!source.parents?.includes(record.head_sha)))refuse('checkout does not bind the requested source');
  if(!log.includes(`Nextest run ID ${report.uuid} with nextest profile: ci`)||!log.includes('cargo nextest run --locked --workspace --profile ci'))refuse('JUnit does not bind the actual Nextest invocation');
  const steps=record.job?.steps?.filter(s=>s.name==='Run tests with observable retry semantics')??[];
  if(steps.length!==1||!inside(report.timestamp,steps[0].started_at,steps[0].completed_at))refuse('JUnit timestamp is outside its execution step');
  const rustc=log.match(/^rustc (.+)$/m)?.[0],host=log.match(/^host: (.+)$/m)?.[1],nextest=log.match(/^cargo-nextest (.+)$/m)?.[0];
  const image=log.match(/##\[group\]Runner Image\r?\nImage: ([^\r\n]+)\r?\nVersion: ([^\r\n]+)/);
  if(!rustc||!host||!nextest||!image)refuse('hosted tool/platform identity missing');
  const summaries=[...log.matchAll(/^\s*Summary\s+\[[^\]\n]+\]\s+(\d+) tests run:\s*(.*)$/gm)];
  if(summaries.length!==1||Number(summaries[0][1])!==report.counts.tests)refuse('JUnit denominator differs from actual Nextest summary');
  const scope={profile:'core-nextest-ci',command:'cargo nextest run --locked --workspace --profile ci',rustc,host,nextest,image:image[1],imageVersion:image[2],cargoVersion:null,config:source.config,sourcePaths:source.sourcePaths,testIdentities:report.cases.map(c=>[c.suite,c.classname,c.name])};
  const clean=report.counts.tests===report.counts.firstAttemptPassed;
  const execution=clean&&record.job.conclusion==='success'&&steps[0].conclusion==='success';
  return {...record,source,report,scope,scope_sha256:nextestHash(JSON.stringify(scope)),execution_passed:execution,first_attempt_passed:record.attempt===1&&execution,state:execution?'qualified':'observed_nonpassing',binding_limit:'Source checkout and tool versions are hosted-log observations. The legacy producer emitted no actual Cargo version, resolved features or test executable hashes.'};
}
export function summarizeNextest(records) {
  const sorted=[...records].sort((a,b)=>a.run_id-b.run_id||a.attempt-b.attempt),seen=new Set(),groups=new Map(),scopes=new Map();
  for(const r of sorted) {
    const key=`${r.run_id}/${r.attempt}`;
    if(r.schema_version!=='opencoven.threads-nextest-retention/v1'||!id(r.run_id)||!id(r.attempt)||!sha(r.head_sha)||seen.has(key)||typeof r.execution_passed!=='boolean'||typeof r.first_attempt_passed!=='boolean'||r.first_attempt_passed&&(r.attempt!==1||!r.execution_passed))refuse('duplicate or contradictory core attempt');
    seen.add(key);const group=groups.get(r.run_id)??[];
    if(group.some(x=>x.head_sha!==r.head_sha||x.event!==r.event))refuse('run source/event changed across attempts');
    group.push(r);groups.set(r.run_id,group);
    const scopeKey=`${r.event}/${r.scope_sha256??'unknown'}`;const scope=scopes.get(scopeKey)??{event:r.event,scope_sha256:r.scope_sha256,attempts:0,first_attempts:0,first_attempt_passes:0,tests:0,first_execution_passes:0,flaky_tests:0,retries:0};
    scope.attempts++;if(r.attempt===1){scope.first_attempts++;scope.first_attempt_passes+=Number(r.first_attempt_passed);if(r.report){scope.tests+=r.report.counts.tests;scope.first_execution_passes+=r.report.counts.firstAttemptPassed;scope.flaky_tests+=r.report.counts.flaky;scope.retries+=r.report.counts.retries;}}
    scopes.set(scopeKey,scope);
  }
  const first=sorted.filter(r=>r.attempt===1);
  return {profile:'core-nextest-ci',observed_runs:groups.size,retained_attempts:sorted.length,first_attempts:first.length,first_attempt_passes:first.filter(r=>r.first_attempt_passed).length,missing_original_attempts:[...groups.values()].filter(g=>!g.some(r=>r.attempt===1)).length,recovered_on_later_attempt:[...groups.values()].filter(g=>g.some(r=>r.attempt===1&&!r.execution_passed)&&g.some(r=>r.attempt>1&&r.execution_passed)).length,scopes:[...scopes.entries()].sort(([a],[b])=>a.localeCompare(b,'en')).map(([,s])=>s),gaps:sorted.filter(r=>!r.report||!r.source||!r.scope_sha256).map(r=>({run_id:r.run_id,attempt:r.attempt,state:r.state})),native_qualification:false,elapsed_window_qualified:false,limitation:'Observed core CI attempts only. Reported test counts in unknown scope remain unbound observations. Missing expected runs, causal flake diagnoses, native results and a complete 30-day retention window are not inferred.'};
}
