import assert from "node:assert/strict";
import test from "node:test";
import {buildWindowLedger,validateWindowDeclaration} from "../lib/daemon-window-ledger.mjs";
import {syntheticWindowControl as fixture} from "./fixtures/native-window-control.mjs";
const build=f=>buildWindowLedger(f.declaration,f.runs,f.records,"2026-02-01T00:00:00Z");

test("synthetic complete control has fixed 90 OS and 9270 journey opportunities",()=>{
  const r=build(fixture());assert.equal(r.window_qualified,true);assert.equal(r.expected_os_observations,90);assert.equal(r.qualified_os_observations,90);assert.equal(r.first_attempt_percent,100);assert.equal(r.journey_rows.length,9270);assert.equal(r.qualified_journeys,9270);assert.equal(r.terminal_windows,180);assert.equal(r.rows[0].arrival_delay_seconds,180);
});
for(const kind of ["missing-day","failed-os","skipped-os","missing-os-evidence","terminal-gap","source-mismatch","lock-mismatch","incomplete-journeys","duplicate-slot","before-slot","missing-first-attempt","manual-substitution"]) test(`${kind} cannot reduce the denominator or qualify`,()=>{
  const f=fixture();
  if(kind === "missing-day") {f.runs.shift();f.records.shift();}
  if(["failed-os","skipped-os","missing-os-evidence"].includes(kind)) Object.assign(f.records[0].rows[0],{state:kind,first_attempt_qualified:false,execution_passed:false});
  if(kind === "terminal-gap") f.records[0].rows[0].terminal_proof.missing=1;
  if(kind === "source-mismatch") f.records[0].rows[0].source_series={...f.records[0].rows[0].source_series,threads_sha:"c".repeat(40)};
  if(kind === "lock-mismatch") f.records[0].rows[0].source_series={...f.records[0].rows[0].source_series,lock_sha256:"c".repeat(64)};
  if(kind === "incomplete-journeys") f.records[0].rows[0].journeys={scenarios:[],manifests:0,junit_cases:0};
  if(kind === "duplicate-slot") f.runs.push({...f.runs[0],id:99});
  if(kind === "before-slot") f.runs[0].created_at=f.records[0].created_at=f.records[0].started_at="2026-01-01T07:46:00Z";
  if(kind === "missing-first-attempt") {f.records[0].attempt=2;f.records[0].first_attempt=false;f.records[0].rows.forEach(r=>r.first_attempt_qualified=false);f.runs[0].run_attempt=2;}
  if(kind === "manual-substitution") f.runs[0].event=f.records[0].event="workflow_dispatch";
  const r=build(f);assert.equal(r.window_qualified,false);assert.equal(r.expected_os_observations,90);assert.ok(r.qualified_os_observations<90);assert.equal(r.journey_rows.length,9270);
});
test("rerun pass remains recovery rather than replacing the first failure",()=>{
  const f=fixture(),retry=structuredClone(f.records[0]);retry.attempt=2;retry.first_attempt=false;retry.rows.forEach(r=>r.first_attempt_qualified=false);f.records.push(retry);f.runs[0].run_attempt=2;
  f.records[0].rows[0].first_attempt_qualified=false;f.records[0].rows[0].execution_passed=false;f.records[0].rows[0].state="job_failure";
  const r=build(f);assert.equal(r.window_qualified,false);assert.equal(r.qualified_os_observations,89);assert.equal(r.recovered_on_rerun,1);
});
test("scope changes require an explicit accepted series and retain per-series denominators",()=>{
  const f=fixture(),profiles=structuredClone(f.declaration.accepted_series[0].profiles);
  for(const profile of Object.values(profiles)) profile.lock_sha256="c".repeat(64);
  f.records[0].rows.forEach(row=>row.source_series=profiles[row.os]);
  assert.equal(build(f).window_qualified,false);
  f.declaration.accepted_series.push({id:"synthetic-reviewed-later-series",profiles});
  const r=build(f);assert.equal(r.window_qualified,true);assert.equal(r.by_series["synthetic-reviewed-later-series"].qualified,3);
});
test("unelapsed windows never qualify",()=>{
  const f=fixture();assert.equal(buildWindowLedger(f.declaration,f.runs,f.records,"2026-01-30T23:59:59Z").window_qualified,false);
});
test("omitted census identities, duplicate attempts and contradictory records refuse",()=>{
  for(const kind of ["omitted-census","duplicate-attempt","changed-event"]) {
    const f=fixture();
    if(kind === "omitted-census") f.runs.shift();
    if(kind === "duplicate-attempt") f.records.push(f.records[0]);
    if(kind === "changed-event") f.records[0].event="workflow_dispatch";
    assert.throws(()=>build(f));
  }
});
test("declaration cannot change the metric, target, inventory, day count or invalid calendar",()=>{
  for(const [key,value] of [["days",29],["minimum_percent",99],["governing_metric","passed_only"],["window_start","2026-02-30"],["scenario_inventory_sha256","f".repeat(64)],["declared_at","2026-01-01T08:00:00Z"],["schedule_utc","08:00"]]) {
    const f=fixture();f.declaration[key]=value;assert.throws(()=>validateWindowDeclaration(f.declaration));
  }
});
