import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { measureCoverage } from "../measure-rust-coverage.mjs";

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "threads-coverage-control-"));
  const git = args => execFileSync("git", args, {cwd:root,encoding:"utf8"}).trim();
  mkdirSync(join(root,"src"));
  writeFileSync(join(root,"src/lib.rs"), "pub fn control() {}\n");
  writeFileSync(join(root,"Cargo.toml"), '[package]\nname="control"\nversion="0.1.0"\nedition="2021"\n');
  writeFileSync(join(root,"Cargo.lock"), "version = 4\n");
  writeFileSync(join(root,"rust-toolchain.toml"), '[toolchain]\nchannel = "1.88.0"\n');
  writeFileSync(join(root,".gitignore"), "/target/\n");
  git(["init","-q"]); git(["add","."]);
  git(["-c","user.name=Synthetic","-c","user.email=synthetic@example.invalid","-c","commit.gpgsign=false","commit","-qm","synthetic source"]);
  const toolroot = join(root,"target/toolchain");
  const bindir = join(toolroot,"lib/rustlib/synthetic-host/bin");
  mkdirSync(bindir,{recursive:true});
  for(const n of ["llvm-cov","llvm-profdata"]) writeFileSync(join(bindir,n), "synthetic tool");
  const output = join(root,"target/measurement");
  const lcov = `SF:${root}/src/lib.rs\nFN:1,control\nFNDA:1,control\nFNF:1\nFNH:1\nDA:1,1\nBRF:0\nBRH:0\nLF:1\nLH:1\nend_of_record\n`;
  const calls = [];
  const run = (command,args,options) => {
    calls.push([command,args]);
    let stdout;
    if(command === "rustc") stdout = args[0] === "--print" ? toolroot : "rustc 1.88.0 (synthetic)\nhost: synthetic-host\nLLVM version: 20.1.5\n";
    else if(command.endsWith("llvm-cov") || command.endsWith("llvm-profdata")) stdout = "LLVM version 20.1.5\n";
    else if(args[0] === "--version") stdout = "cargo 1.88.0 (synthetic)\n";
    else if(args[0] === "llvm-cov" && args[1] === "--version") stdout = "cargo-llvm-cov 0.9.1\n";
    else if(args[0] === "llvm-cov") {
      writeFileSync(args.at(-1),args.includes("--json") ? JSON.stringify({type:"llvm.coverage.json.export",version:"2.0.1",data:[{files:[{filename:join(root,"src/lib.rs"),summary:{functions:{count:1,covered:1},instantiations:{count:1,covered:1},lines:{count:1,covered:1},branches:{count:0,covered:0}}}]}]}) : lcov);
      writeFileSync(options.stdio[1], "synthetic measurement output\n");
      stdout = "";
    } else if(args[0] === "metadata") stdout = JSON.stringify({workspace_members:["control"],packages:[{id:"control",name:"control",version:"0.1.0",source:null}],resolve:{nodes:[{id:"control",features:[]}]}});
    else throw Error(`unexpected ${command} ${args}`);
    return {status:0,stdout,stderr:""};
  };
  t.after(() => rmSync(root,{recursive:true,force:true}));
  return {root,output,run,env:{},calls};
}

test("source-bound runner retains original bytes, tools, features and compatible scope", t => {
  const f = fixture(t), receipt = measureCoverage(f);
  assert.equal(receipt.qualified,true);
  assert.equal(receipt.source.commit.length,40);
  assert.equal(receipt.source.tree.length,40);
  assert.equal(receipt.coverage.totals.lines.total,1);
  assert.equal(receipt.tools.cargoLlvmCov,"cargo-llvm-cov 0.9.1");
  assert.equal(receipt.scopeSha256.length,64);
  assert.equal(receipt.files.find(v => v.path === "lcov.info").sha256.length,64);
  assert.deepEqual(receipt.resolvedFeatures,[{name:"control",version:"0.1.0",source:null,workspace:true,features:[]}]);
  assert.match(readFileSync(join(f.output,"measurement.log"),"utf8"),/synthetic measurement/);
  assert.deepEqual(JSON.parse(readFileSync(join(f.output,"measurement.json"),"utf8")),receipt);
  assert.throws(() => measureCoverage(f),/exist/i);
});

test("wrong tool and inherited instrumentation refuse before measurement", t => {
  for(const kind of ["tool", "flags", "target", "profile", "llvm", "LLVM_COV_FLAGS", "LLVM_PROFDATA_FLAGS", "LLVM_PROFILE_FILE_NAME"]) {
    const f=fixture(t), run=f.run;
    if(kind === "tool") f.run=(cmd,args,opts) => cmd === "cargo" && args[0] === "llvm-cov" && args[1] === "--version" ? {status:0,stdout:"cargo-llvm-cov 0.9.2"} : run(cmd,args,opts);
    else f.env[{flags:"RUSTFLAGS",target:"CARGO_BUILD_TARGET",profile:"CARGO_PROFILE_TEST_OPT_LEVEL",llvm:"LLVM_COV"}[kind] ?? kind]="override";
    assert.throws(() => measureCoverage(f));
    assert.equal(JSON.parse(readFileSync(join(f.output,"measurement.json"),"utf8")).qualified,false);
    assert.equal(f.calls.some(([,a])=>a.includes("--lcov")),false);
  }
});

test("original probe diagnostics and earlier successful tool identities survive failure", t => {
  const f=fixture(t), run=f.run;
  f.run=(cmd,args,opts)=>cmd === "cargo" && args[0] === "llvm-cov" ? {status:73,stdout:"original stdout\n",stderr:"original stderr\n",signal:null}:run(cmd,args,opts);
  assert.throws(()=>measureCoverage(f));
  const r=JSON.parse(readFileSync(join(f.output,"measurement.json"),"utf8"));
  assert.match(r.tools.rustc,/1.88.0/);
  const probes=JSON.parse(readFileSync(join(f.output,"probes.json"),"utf8"));
  assert.equal(probes.at(-1).status,73);
  assert.equal(probes.at(-1).stdout,"original stdout\n");
  assert.equal(probes.at(-1).stderr,"original stderr\n");
  assert.ok(r.files.some(v=>v.path === "probes.json"));
});

test("metadata is limited to actual host and keeps only resolved packages", t => {
  const f=fixture(t), run=f.run;
  f.run=(cmd,args,opts)=>{
    if(args[0] === "metadata") {
      assert.deepEqual(args.slice(-2),["--filter-platform","synthetic-host"]);
      const r=run(cmd,args,opts), m=JSON.parse(r.stdout);
      m.packages.push({id:"other-platform",name:"foreign",version:"1.0.0",source:null});
      return {...r,stdout:JSON.stringify(m)};
    }
    return run(cmd,args,opts);
  };
  const r=measureCoverage(f);
  assert.equal(r.resolvedFeatures.length,1);
  assert.ok(r.metadataCommand.includes("synthetic-host"));
});

test("failed measurement preserves partial LCOV and original exit status", t => {
  const f=fixture(t), run=f.run;
  f.run=(cmd,args,opts) => {const r=run(cmd,args,opts);return args.includes("--lcov")?{...r,status:7}:r;};
  assert.throws(()=>measureCoverage(f));
  const r=JSON.parse(readFileSync(join(f.output,"measurement.json"),"utf8"));
  assert.equal(r.qualified,false);
  assert.equal(r.command.status,7);
  assert.ok(r.files.some(v=>v.path === "lcov.info"));
});

test("source mutation and malformed LCOV never qualify", t => {
  for(const kind of ["dirty-before","dirty-after","bad-lcov"]) {
    const f=fixture(t), run=f.run;
    if(kind === "dirty-before") writeFileSync(join(f.root,"src/lib.rs"),"changed\n");
    f.run=(cmd,args,opts) => {
      const r=run(cmd,args,opts);
      if(args.includes("--lcov")) {
        if(kind === "dirty-after") writeFileSync(join(f.root,"src/lib.rs"),"changed\n");
        if(kind === "bad-lcov") writeFileSync(args.at(-1),"SF:unknown\n");
      }
      return r;
    };
    assert.throws(()=>measureCoverage(f));
    assert.equal(JSON.parse(readFileSync(join(f.output,"measurement.json"),"utf8")).qualified,false);
  }
});

test("existing informational coverage job pins tools and retains exact attempts", () => {
  const workflow=readFileSync(new URL("../../.github/workflows/ci.yml",import.meta.url),"utf8").split("  coverage:\n")[1];
  assert.match(workflow,/continue-on-error: true/);
  assert.match(workflow,/tool: cargo-llvm-cov@0\.9\.1/);
  assert.match(workflow,/node-version: 24\.22\.0/);
  assert.match(workflow,/node scripts\/measure-rust-coverage\.mjs/);
  assert.match(workflow,/rust-coverage-\$\{\{ github.run_id \}\}-\$\{\{ github.run_attempt \}\}/);
  assert.match(workflow,/retention-days: 45/);
  assert.match(workflow,/artifact-digest/);
});

test("only matching source/tool/feature/configuration scopes are comparable", t => {
  const first=fixture(t), a=measureCoverage(first);
  const changed=fixture(t);
  writeFileSync(join(changed.root,"src/lib.rs"),"pub fn control() {}\n// changed source\n");
  execFileSync("git",["add","."],{cwd:changed.root});
  execFileSync("git",["-c","user.name=Synthetic","-c","user.email=synthetic@example.invalid","-c","commit.gpgsign=false","commit","-qm","changed source"],{cwd:changed.root});
  const b=measureCoverage(changed);
  assert.notEqual(a.source.tree,b.source.tree);
  assert.equal(a.scopeSha256,b.scopeSha256);
  const feature=fixture(t), run=feature.run;
  feature.run=(cmd,args,opts)=>{
    const r=run(cmd,args,opts);
    if(args[0] === "metadata") {const m=JSON.parse(r.stdout);m.resolve.nodes[0].features=["changed"];r.stdout=JSON.stringify(m);}
    return r;
  };
  assert.notEqual(a.scopeSha256,measureCoverage(feature).scopeSha256);
});

test("the pinned CI incremental=0 setting qualifies in its own explicit scope", t => {
  const normal=fixture(t), baseline=measureCoverage(normal);
  const ci=fixture(t);ci.env.CARGO_INCREMENTAL="0";
  const r=measureCoverage(ci);
  assert.equal(r.qualified,true);
  assert.deepEqual(r.scope.environment,{cargoIncremental:"0"});
  assert.notEqual(r.scopeSha256,baseline.scopeSha256);
  for(const value of ["1","true","unknown"]) {
    const f=fixture(t);f.env.CARGO_INCREMENTAL=value;
    assert.throws(()=>measureCoverage(f),/override/);
  }
});
