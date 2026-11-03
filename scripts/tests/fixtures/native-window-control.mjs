import {createHash} from "node:crypto";
import {readFileSync} from "node:fs";
const bytes=readFileSync(new URL("../../../e2e/native-daemon-scenarios.json",import.meta.url)),inventory=JSON.parse(bytes);
const runners=["ubuntu-24.04","macos-15","windows-2022"],platforms=["linux","darwin","win32"];
const cases=i=>[...inventory.common,...(i<2?inventory.unix:[]),...(i===0?inventory.linux:[])].sort();
export function syntheticWindowControl() {
  const profiles=Object.fromEntries(runners.map((os,i)=>[os,{coven_sha:inventory.coven_ref,threads_sha:"a".repeat(40),lock_sha256:"b".repeat(64),platform:platforms[i],arch:"x64",rustc:"rustc 1.95.0 (synthetic)",cargo:"cargo 1.95.0 (synthetic)",node:"v24.22.0"}]));
  const declaration={schema_version:"opencoven.threads-native-window/v1",repository:"OpenCoven/coven-threads",workflow:".github/workflows/daemon-reliability.yml",window_start:"2026-01-01",days:30,schedule_utc:"07:47",governing_metric:"scheduled_os_first_attempts",minimum_percent:99.5,declared_at:"2025-12-31T00:00:00Z",declaration_comment_id:42,scenario_inventory_sha256:createHash("sha256").update(bytes).digest("hex"),accepted_series:[{id:"synthetic-series",profiles}]};
  const runs=Array.from({length:30},(_,day)=>({id:day+1,run_attempt:1,head_sha:"a".repeat(40),event:"schedule",path:declaration.workflow,repository:{full_name:declaration.repository},created_at:`2026-01-${String(day+1).padStart(2,"0")}T07:50:00Z`,status:"completed",conclusion:"success"}));
  const records=runs.map(run=>({run_id:run.id,attempt:1,first_attempt:true,threads_sha:run.head_sha,event:run.event,created_at:run.created_at,started_at:run.created_at,rows:runners.map((os,i)=>({os,state:"qualified",execution_passed:true,first_attempt_qualified:true,artifact_state:"verified_evidence",source_series:profiles[os],journeys:{scenarios:cases(i),manifests:cases(i).length,junit_cases:cases(i).length},terminal_proof:{qualified:true,captures:1,windows:2,duplicates:0,missing:0}}))}));
  return {declaration,runs,records};
}
