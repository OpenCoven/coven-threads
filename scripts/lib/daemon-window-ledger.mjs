import {createHash} from "node:crypto";
import {readFileSync} from "node:fs";
import {isDeepStrictEqual} from "node:util";
import {nativeRunners} from "./daemon-retention.mjs";

const inventoryBytes=readFileSync(new URL("../../e2e/native-daemon-scenarios.json",import.meta.url));
const inventory=JSON.parse(inventoryBytes),dayMs=86400000;
const hash=bytes=>createHash("sha256").update(bytes).digest("hex");
const refuse=message=>{throw new Error(`native window refused: ${message}`);};
const sha=s=>typeof s === "string" && /^[0-9a-f]{40}$/.test(s);
const digest=s=>typeof s === "string" && /^[0-9a-f]{64}$/.test(s);
const id=n=>Number.isSafeInteger(n) && n>0;
function utc(value) {
  if(typeof value !== "string" || !/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{3})?Z$/.test(value)) refuse("explicit UTC timestamp required");
  const time=Date.parse(value);
  if(!Number.isFinite(time) || new Date(time).toISOString() !== value.replace(/(?<!\.\d{3})Z$/,".000Z")) refuse("invalid UTC calendar");
  return time;
}
const scenarios=i=>[...inventory.common,...(i<2?inventory.unix:[]),...(i===0?inventory.linux:[])].sort();

export function validateWindowDeclaration(d) {
  if(d?.schema_version !== "opencoven.threads-native-window/v1" || d.repository !== "OpenCoven/coven-threads" || d.workflow !== ".github/workflows/daemon-reliability.yml" || d.days !== 30 || d.schedule_utc !== "07:47" || d.governing_metric !== "scheduled_os_first_attempts" || d.minimum_percent !== 99.5 || d.scenario_inventory_sha256 !== hash(inventoryBytes) || !/^\d{4}-\d\d-\d\d$/.test(d.window_start ?? "")) refuse("missing or changed window contract");
  const start=utc(d.window_start+"T00:00:00Z"),declared=utc(d.declared_at);
  if(declared>=start || !id(d.declaration_comment_id) || !Array.isArray(d.accepted_series) || !d.accepted_series.length) refuse("prior declaration and explicit accepted source series required");
  const names=new Set();
  for(const series of d.accepted_series) {
    if(!/^[A-Za-z0-9._-]+$/.test(series.id ?? "") || names.has(series.id) || !isDeepStrictEqual(Object.keys(series.profiles ?? {}).sort(),[...nativeRunners].sort())) refuse("ambiguous source series");
    names.add(series.id);
    const heads=new Set();
    nativeRunners.forEach((os,i)=>{
      const p=series.profiles[os];
      if(!p || !isDeepStrictEqual(Object.keys(p).sort(),["arch","cargo","coven_sha","lock_sha256","node","platform","rustc","threads_sha"]) || p.coven_sha !== inventory.coven_ref || !sha(p.threads_sha) || !digest(p.lock_sha256) || p.platform !== ["linux","darwin","win32"][i] || !/^[a-z0-9_-]+$/.test(p.arch ?? "") || p.node !== "v24.22.0" || !/^rustc 1\.95\.0 /.test(p.rustc ?? "") || !/^cargo 1\.95\.0 /.test(p.cargo ?? "")) refuse("incomplete source/tool/platform profile");
      heads.add(p.threads_sha);
    });
    if(heads.size !== 1) refuse("series mixes Threads heads");
  }
  return {start,end:start+30*dayMs,lastDate:new Date(start+29*dayMs).toISOString().slice(0,10)};
}

export function buildWindowLedger(declaration,censusRuns,verifiedRecords,serverObservedAt) {
  const {start,end}=validateWindowDeclaration(declaration),observed=utc(serverObservedAt);
  if(utc(declaration.declared_at)>observed || !Array.isArray(censusRuns) || !Array.isArray(verifiedRecords)) refuse("invalid observation inputs");
  const runs=new Map(),byDay=new Map(),excluded=[];
  for(const run of censusRuns) {
    if(!id(run.id) || runs.has(run.id) || !sha(run.head_sha) || !id(run.run_attempt) || run.repository?.full_name !== declaration.repository || run.path !== declaration.workflow || !["schedule","workflow_dispatch"].includes(run.event)) refuse("invalid, duplicate or foreign census run");
    const created=utc(run.created_at);
    if(created<start || created>=end || created>observed) refuse("census run outside authenticated range/time");
    runs.set(run.id,run);
    if(run.event === "workflow_dispatch") {excluded.push({run_id:run.id,event:run.event,state:"manual_excluded"});continue;}
    const date=new Date(created).toISOString().slice(0,10),group=byDay.get(date) ?? [];
    group.push(run);byDay.set(date,group);
  }
  const records=new Map(),identities=new Set();let outsideRecords=0;
  for(const record of verifiedRecords) {
    if(!id(record.run_id) || !id(record.attempt) || identities.has(`${record.run_id}/${record.attempt}`) || !sha(record.threads_sha) || !Array.isArray(record.rows) || !isDeepStrictEqual(record.rows.map(row=>row.os).sort(),[...nativeRunners].sort()) || record.first_attempt !== (record.attempt === 1)) refuse("invalid or duplicate retained attempt");
    identities.add(`${record.run_id}/${record.attempt}`);
    const created=utc(record.created_at);
    if(created<start || created>=end) {outsideRecords++;continue;}
    const run=runs.get(record.run_id);
    if(!run || run.head_sha !== record.threads_sha || run.event !== record.event || run.created_at !== record.created_at || record.attempt>run.run_attempt) refuse("retained attempt contradicts complete census");
    if(record.started_at !== null && utc(record.started_at)<created) refuse("attempt starts before creation");
    const group=records.get(record.run_id) ?? [];group.push(record);records.set(record.run_id,group);
  }
  const result={schema_version:"opencoven.threads-native-window-ledger/v1",declaration_sha256:hash(JSON.stringify(declaration)),window_start:new Date(start).toISOString(),window_end_exclusive:new Date(end).toISOString(),server_observed_at:new Date(observed).toISOString(),elapsed_complete:observed>=end,governing_metric:declaration.governing_metric,minimum_percent:99.5,expected_os_observations:90,qualified_os_observations:0,first_attempt_percent:0,expected_journeys:0,qualified_journeys:0,terminal_windows:0,recovered_on_rerun:0,rows:[],journey_rows:[],excluded_runs:excluded.sort((a,b)=>a.run_id-b.run_id),outside_window_retained_attempts:outsideRecords,by_series:{},window_qualified:false,human_acceptance_inferred:false};
  for(let day=0;day<30;day++) {
    const midnight=start+day*dayMs,date=new Date(midnight).toISOString().slice(0,10),slot=midnight+(7*60+47)*60000;
    const candidates=byDay.get(date) ?? [],run=candidates.length === 1 ? candidates[0] : null;
    const attempts=run ? records.get(run.id) ?? [] : [],first=attempts.find(a=>a.attempt === 1);
    nativeRunners.forEach((os,i)=>{
      const expected=scenarios(i),row=first?.rows.find(r=>r.os === os);
      const arrival=run ? (utc(run.created_at)-slot)/1000 : null;
      const matches=row?.source_series ? declaration.accepted_series.filter(s=>isDeepStrictEqual(s.profiles[os],row.source_series)) : [];
      const terminal=row?.terminal_proof;
      let state="qualified";
      if(candidates.length === 0) state="missing_scheduled_run";
      else if(candidates.length>1) state="ambiguous_schedule_slot";
      else if(arrival<0) state="before_schedule_unsupported";
      else if(!first) state="missing_first_attempt";
      else if(!row.first_attempt_qualified || !row.execution_passed || row.state !== "qualified" || row.artifact_state !== "verified_evidence") state=row.state === "qualified" ? "unqualified_first_attempt" : row.state;
      else if(matches.length !== 1 || row.source_series.threads_sha !== first.threads_sha) state="unaccepted_or_ambiguous_source_profile";
      else if(!row.journeys || row.journeys.manifests !== expected.length || row.journeys.junit_cases !== expected.length || !isDeepStrictEqual(row.journeys.scenarios,expected)) state="incomplete_journeys";
      else if(terminal?.qualified !== true || !id(terminal.windows) || !id(terminal.captures) || terminal.missing !== 0 || terminal.duplicates !== 0) state="terminal_evidence_unqualified";
      const qualified=state === "qualified",series=matches.length === 1 ? matches[0].id : null;
      const recovered=!qualified && attempts.some(a=>a.attempt>1 && a.rows.some(r=>r.os === os && r.execution_passed === true));
      result.rows.push({date,slot:new Date(slot).toISOString(),os,state,qualified,run_id:run?.id ?? null,candidate_run_ids:candidates.map(r=>r.id).sort((a,b)=>a-b),source:run?.head_sha ?? null,series,arrival_delay_seconds:arrival,start_delay_seconds:first?.started_at ? (utc(first.started_at)-slot)/1000 : null,retained_attempts:attempts.map(a=>a.attempt).sort((a,b)=>a-b),recovered_on_rerun:recovered,expected_journeys:expected.length,qualified_journeys:qualified?expected.length:0,terminal_windows:qualified?terminal.windows:0});
      result.expected_journeys+=expected.length;
      if(qualified) {result.qualified_os_observations++;result.qualified_journeys+=expected.length;result.terminal_windows+=terminal.windows;}
      if(recovered) result.recovered_on_rerun++;
      const counters=result.by_series[series ?? "unaccepted_or_missing"] ??= {observed:0,qualified:0};counters.observed++;if(qualified) counters.qualified++;
      for(const scenario of expected) result.journey_rows.push({date,os,scenario,run_id:run?.id ?? null,series,qualified,state});
    });
  }
  result.first_attempt_percent=result.qualified_os_observations*100/90;
  result.window_qualified=result.elapsed_complete && result.qualified_os_observations*1000>=90*995;
  return result;
}
