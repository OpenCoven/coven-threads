import { createHash } from "node:crypto";
import { existsSync, lstatSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const refuse=message=>{throw new Error(`terminal capture refused: ${message}`);};
const regular=path=>{if(!lstatSync(path).isFile()) refuse("non-regular evidence");return readFileSync(path);};
const hash=bytes=>createHash("sha256").update(bytes).digest("hex");
const terminals=new Set(["proposal_approved","proposal_vetoed","proposal_rejected"]);
// Read-only evidence checks mirror approval.rs's typed detail and
// audit.rs::validate_window_close_detail; they grant no runtime authority.
const closeRules=new Map([
  ["applied",["proposal_approved",true]],
  ["vetoed",["proposal_vetoed",null]],
  ["evidence_diverged",["proposal_rejected",false]],
  ["revalidation_failed",["proposal_rejected",false]],
  ["superseded",["proposal_rejected",null]],
]);

export function qualifyTerminalCaptures(root) {
  if(!lstatSync(root).isDirectory()) refuse("missing journey directory");
  const result={qualified:true,scope:"retained ward-window-census captures; explicit-supersession required",captures:0,windows:0,missing:0,duplicates:0,sources:[]};
  const scenarios=new Set();let required=false;
  for(const entry of readdirSync(root).sort()) {
    const directory=join(root,entry);
    if(!lstatSync(directory).isDirectory()) refuse("non-directory journey");
    const manifest=JSON.parse(regular(join(directory,"manifest.json")));
    if(typeof manifest.scenario !== "string" || scenarios.has(manifest.scenario) || manifest.result !== "passed") refuse("ambiguous or failed journey");
    scenarios.add(manifest.scenario);
    const censusPath=join(directory,"state/ward-window-census.json");
    if(!existsSync(censusPath)) {if(manifest.scenario === "explicit-supersession") refuse("required terminal capture missing");continue;}
    const censusBytes=regular(censusPath), auditBytes=regular(join(directory,"state/ward-audit.jsonl"));
    const census=JSON.parse(censusBytes), lines=auditBytes.toString("utf8").trim().split("\n");
    if(census.complete !== true || !Array.isArray(census.windows) || census.windows.length === 0 || !auditBytes.length) refuse("incomplete census/audit");
    const rows=lines.map(line=>JSON.parse(line));
    if(rows.some(row=>!Number.isSafeInteger(row.id) || row.id <= 0 || typeof row.event_type !== "string") || new Set(rows.map(row=>row.id)).size !== rows.length) refuse("ambiguous audit rows");
    const ids=new Set();
    for(const window of census.windows) {
      if(typeof window.proposalId !== "string" || !window.proposalId || ids.has(window.proposalId) || window.classification !== "typed_terminal_recorded" || typeof window.closeReason !== "string" || !window.closeReason) refuse("unknown or duplicate census window");
      ids.add(window.proposalId);
      const own=rows.filter(row=>row.proposal_id === window.proposalId);
      const opens=own.filter(row=>row.event_type === "proposal_window_opened"), closes=own.filter(row=>terminals.has(row.event_type));
      if(opens.length !== 1 || closes.length !== 1 || opens[0].id>=closes[0].id || own.some(row=>row.event_type === "proposal_expired" || row.decision === "proposal-expired")) refuse("missing/duplicate/reversed/unsupported terminal chain");
      const detail=JSON.parse(closes[0].detail);
      const close=closes[0].event_type === "proposal_approved" ? detail.window_close : detail;
      const rule=closeRules.get(close?.reason);
      if(!close || typeof close !== "object" || Array.isArray(close) || !rule || closes[0].event_type !== rule[0] || (close.replay_hash_matched ?? null) !== rule[1] || (close.rationale !== null && close.rationale !== undefined && typeof close.rationale !== "string") || close.reason !== window.closeReason) refuse("typed close family/payload/replay contradicts canonical contract or census");
    }
    if(rows.some(row=>row.event_type === "proposal_window_opened" && !ids.has(row.proposal_id))) refuse("census omits an opened window");
    if(manifest.scenario === "explicit-supersession") {
      if(census.windows.length !== 2 || !["applied","superseded"].every(reason=>census.windows.some(w=>w.closeReason === reason))) refuse("required supersession terminals missing");
      required=true;
    }
    result.captures++;result.windows+=ids.size;
    result.sources.push({scenario:manifest.scenario,census_sha256:hash(censusBytes),audit_sha256:hash(auditBytes)});
  }
  if(!required || !result.windows) refuse("no required nonzero terminal scope");
  return result;
}
