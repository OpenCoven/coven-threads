import assert from "node:assert/strict";
import test from "node:test";
import { parseLcov } from "../lib/rust-coverage.mjs";

const root = "/synthetic-checkout";
const sources = ["a", "b", "c"].map(n => ({path: `src/${n}.rs`, sha256: "a".repeat(64), lines: 20}));
const block = (path = "src/a.rs") => `SF:${root}/${path}\nFN:2,name,with,commas\nFNDA:3,name,with,commas\nFNF:1\nFNH:1\nDA:2,3\nDA:3,0\nBRDA:2,0,0,3\nBRDA:2,0,1,-\nBRF:2\nBRH:1\nLF:2\nLH:1\nend_of_record\n`;
const parse = text => parseLcov(text, {root, sources});

test("qualifies all source records and recomputes covered/total denominators", () => {
  const r = parse(block() + block("src/b.rs"));
  assert.deepEqual(r.totals.lines, {covered:2,total:4,percent:50});
  assert.deepEqual(r.totals.functions, {covered:2,total:2,percent:100});
  assert.deepEqual(r.totals.branches, {covered:2,total:4,percent:50});
  assert.deepEqual(r.unreported, ["src/c.rs"]);
  assert.equal(r.files[0].path, "src/a.rs");
});

test("zero-denominator files stay explicit and unsupported branch coverage is null", () => {
  const zero = `SF:src/b.rs\nFNF:0\nFNH:0\nLF:0\nLH:0\nend_of_record\n`;
  const r = parse(block() + zero);
  assert.equal(r.files[1].lines.percent, null);
  assert.equal(r.files[1].functions.percent, null);
  assert.equal(r.files[1].branches, null);
  assert.equal(r.totals.branches, null); // mixed branch scopes cannot be summed as complete
});

test("record order and CRLF do not change qualification", () => {
  const reordered = block().replace("FNF:1\n", "").replace("end_of_record", "FNF:1\nend_of_record");
  assert.deepEqual(parse(reordered.replaceAll("\n", "\r\n")), parse(block()));
});

for (const [name, change] of [
  ["empty", () => ""],
  ["missing close", s => s.replace("end_of_record", "")],
  ["missing line summary", s => s.replace("LF:2\n", "")],
  ["missing function summary", s => s.replace("FNF:1\n", "")],
  ["missing branch summary", s => s.replace("BRF:2\n", "")],
  ["duplicate source", s => s + s],
  ["duplicate line", s => s.replace("LF:2", "DA:2,3\nLF:2")],
  ["conflicting line", s => s.replace("LF:2", "DA:2,0\nLF:2")],
  ["duplicate function", s => s.replace("FNF:1", "FN:2,name,with,commas\nFNF:1")],
  ["duplicate function hit", s => s.replace("FNF:1", "FNDA:3,name,with,commas\nFNF:1")],
  ["unbound function", s => s.replace("FNDA:3,name,with,commas", "FNDA:3,other")],
  ["missing function hit", s => s.replace("FNDA:3,name,with,commas\n", "")],
  ["duplicate summary", s => s.replace("LF:2", "LF:2\nLF:2")],
  ["false line total", s => s.replace("LF:2", "LF:3")],
  ["false hit total", s => s.replace("FNH:1", "FNH:0")],
  ["false branch total", s => s.replace("BRF:2", "BRF:3")],
  ["duplicate branch", s => s.replace("BRF:2", "BRDA:2,0,0,3\nBRF:2")],
  ["unsafe integer", s => s.replace("DA:2,3", "DA:2,9007199254740992")],
  ["negative count", s => s.replace("DA:2,3", "DA:2,-1")],
  ["fraction", s => s.replace("DA:2,3", "DA:2,1.5")],
  ["line zero", s => s.replace("DA:2,3", "DA:0,3")],
  ["line past source", s => s.replace("DA:2,3", "DA:21,3")],
  ["function past source", s => s.replace("FN:2,", "FN:21,")],
  ["outside source", s => s.replace(`${root}/src/a.rs`, "/other/src/a.rs")],
  ["untracked source", s => s.replace("src/a.rs", "src/z.rs")],
  ["traversal", s => s.replace("src/a.rs", "src/../src/a.rs")],
  ["alias path", s => s.replace("src/a.rs", "src//a.rs")],
  ["unknown tag", s => s.replace("LF:2", "UNKNOWN:1\nLF:2")],
  ["summary-only", s => s.replace(/^FN.*\n/gm, "").replace("DA:2,3\nDA:3,0\n", "")],
  ["zero covered", s => s.replaceAll("FNDA:3", "FNDA:0").replace("FNH:1", "FNH:0").replace("DA:2,3", "DA:2,0").replace("LH:1", "LH:0")],
  ["unterminated source", s => s + "SF:src/b.rs\n"],
]) test(`refuses ${name}`, () => assert.throws(() => parse(change(block()))));

test("real generic export distinguishes functions from instantiations", () => {
  // Pinned rustc 1.88.0 generic<T> called as u8/u16, exported with Apple LLVM 21.
  // LLVM 20's exporter/report source has the same function-group semantics.
  const lcov = `SF:src/a.rs\nFN:1,_RINvCsieHBN5Fwi9J_7generic7generichEB2_\nFN:1,_RINvCsieHBN5Fwi9J_7generic7generictEB2_\nFN:2,_RNvCsieHBN5Fwi9J_7generic4main\nFNDA:1,_RINvCsieHBN5Fwi9J_7generic7generichEB2_\nFNDA:1,_RINvCsieHBN5Fwi9J_7generic7generictEB2_\nFNDA:1,_RNvCsieHBN5Fwi9J_7generic4main\nFNF:2\nFNH:2\nDA:1,2\nDA:2,1\nBRF:0\nBRH:0\nLF:2\nLH:2\nend_of_record\n`;
  const summary={functions:{count:2,covered:2},instantiations:{count:3,covered:3},lines:{count:2,covered:2},branches:{count:0,covered:0}};
  const llvm={type:"llvm.coverage.json.export",version:"2.0.1",data:[{files:[{filename:"src/a.rs",segments:[[1,1,2,true,true,false],[1,50,0,false,false,false],[2,1,1,true,true,false],[2,10,0,false,false,false]],summary}]}]};
  const r=parseLcov(lcov,{root,sources,llvm});
  assert.deepEqual(parseLcov(lcov,{root,sources,llvm:{...llvm,version:"3.0.1"}}),r);
  assert.deepEqual(r.totals.functions,{total:2,covered:2,percent:100});
  assert.deepEqual(r.totals.instantiations,{total:3,covered:3,percent:100});
  for(const bad of [
    {...llvm,data:[{files:[]}]},
    {...llvm,data:[{files:[...llvm.data[0].files,...llvm.data[0].files]}]},
    {...llvm,version:"unsupported"},
    {...llvm,data:[{files:[{filename:"src/a.rs",summary:{...summary,instantiations:{count:2,covered:2}}}]}]},
    {...llvm,data:[{files:[{filename:"src/a.rs",summary:{...summary,functions:{count:1,covered:1}}}]}]},
  ]) assert.throws(()=>parseLcov(lcov,{root,sources,llvm:bad}));
});

test("real shared-line export keeps LLVM group-lines separate from unique DA lines", () => {
  // rustc 1.88.0: fn a() {} fn main() { a(); }, Apple LLVM 21 original semantics.
  const lcov=`SF:src/a.rs\nFN:1,a\nFN:1,main\nFNDA:1,a\nFNDA:1,main\nFNF:2\nFNH:2\nDA:1,1\nBRF:0\nBRH:0\nLF:2\nLH:2\nend_of_record\n`;
  const file={filename:"src/a.rs",segments:[[1,1,1,true,true,false],[1,7,0,false,false,false],[1,9,1,true,true,false],[1,10,0,false,false,false],[1,11,1,true,true,false],[1,20,0,false,false,false],[1,23,1,true,true,false],[1,24,0,false,false,false],[1,28,1,true,true,false],[1,29,0,false,false,false]],summary:{lines:{count:2,covered:2},functions:{count:2,covered:2},instantiations:{count:2,covered:2},branches:{count:0,covered:0}}};
  const llvm={type:"llvm.coverage.json.export",version:"3.0.1",data:[{files:[file]}]};
  const r=parseLcov(lcov,{root,sources,llvm});
  assert.equal(r.totals.lines.total,1);
  assert.equal(r.totals.llvmLines.total,2);
  for(const corrupted of [lcov.replace("DA:1,1\n",""),lcov.replace("DA:1,1","DA:1,2"),lcov.replace("LF:2","LF:3")]) assert.throws(()=>parseLcov(corrupted,{root,sources,llvm}));
  for(const segments of [[],null,[...file.segments,file.segments[0]],[[1,1,1,"true",true,false]]]) assert.throws(()=>parseLcov(lcov,{root,sources,llvm:{...llvm,data:[{files:[{...file,segments}]}]}}));
});

test("paired segments authenticate wrapped, skipped and gap line counts", () => {
  const lcov=`SF:src/a.rs\nFN:1,f\nFNDA:3,f\nFNF:1\nFNH:1\nDA:1,3\nDA:3,4\nDA:4,4\nDA:5,4\nDA:7,0\nBRF:0\nBRH:0\nLF:5\nLH:4\nend_of_record\n`;
  const segments=[[1,1,3,true,true,false],[2,1,0,false,true,false],[3,1,4,true,true,false],[5,1,0,false,false,false],[6,1,0,false,true,false],[7,1,0,true,true,true],[8,1,0,false,true,false]];
  const summary={lines:{count:5,covered:4},functions:{count:1,covered:1},instantiations:{count:1,covered:1},branches:{count:0,covered:0}};
  const llvm={type:"llvm.coverage.json.export",version:"2.0.1",data:[{files:[{filename:"src/a.rs",segments,summary}]}]};
  assert.equal(parseLcov(lcov,{root,sources,llvm}).totals.lines.total,5);
  assert.throws(()=>parseLcov(lcov.replace("DA:4,4\n",""),{root,sources,llvm}));
});
