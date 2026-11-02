#!/usr/bin/env node
import { execFileSync, spawnSync } from "node:child_process";
import { closeSync, existsSync, lstatSync, mkdirSync, openSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { parseLcov, sha256 } from "./lib/rust-coverage.mjs";

const fail = message => { throw new Error(message); };
const json = value => JSON.stringify(value, null, 2) + "\n";
function git(root, args) { return execFileSync("git",args,{cwd:root,encoding:"utf8",maxBuffer:16*1024*1024}).trimEnd(); }
function snapshot(root) {
  if (git(root,["status","--porcelain=v1","--untracked-files=all"])) fail("coverage requires a clean committed checkout");
  const files = git(root,["ls-files","-z"]).split("\0").filter(Boolean);
  const source = {commit:git(root,["rev-parse","HEAD"]),tree:git(root,["rev-parse","HEAD^{tree}"])};
  const digest = path => {
    if (!lstatSync(join(root,path)).isFile()) fail("source/config must be regular files");
    const bytes=readFileSync(join(root,path));
    return {path,sha256:sha256(bytes),lines:bytes.length ? bytes.toString("utf8").split("\n").length - (bytes.at(-1) === 10 ? 1 : 0) : 0};
  };
  source.rustFiles=files.filter(p=>p.endsWith(".rs")).sort().map(digest);
  source.config=files.filter(p=>/(^|\/)(Cargo\.(toml|lock)|rust-toolchain\.toml|config(?:\.toml)?)$/.test(p) || [".node-version",".github/workflows/ci.yml","scripts/measure-rust-coverage.mjs","scripts/lib/rust-coverage.mjs"].includes(p)).sort().map(digest);
  if(!source.rustFiles.length || !source.config.some(f=>f.path === "Cargo.lock")) fail("missing Rust source or lockfile");
  return source;
}

function rejectOverrides(root,env) {
  for(const [key,value] of Object.entries(env)) {
    if(value && /^(?:RUSTFLAGS|RUSTDOCFLAGS|CARGO_ENCODED_RUSTFLAGS|CARGO_ENCODED_RUSTDOCFLAGS|LLVM_PROFILE_FILE(?:_NAME)?|LLVM_COV(?:_FLAGS)?|LLVM_PROFDATA(?:_FLAGS)?|CARGO_INCREMENTAL|RUSTC|RUSTDOC|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTC_BOOTSTRAP)$|^CARGO_(?:BUILD|TARGET|PROFILE|LLVM_COV)_/.test(key)) fail(`unsupported coverage override: ${key}`);
  }
  // Cargo searches ancestor and user configuration. Do not silently inherit it.
  for(let dir=dirname(root);;dir=dirname(dir)) {
    if(["config","config.toml"].some(p=>existsSync(join(dir,".cargo",p)))) fail("external Cargo configuration is outside the measured scope");
    if(dirname(dir) === dir) break;
  }
  const cargoHome=env.CARGO_HOME || (env.HOME ? join(env.HOME,".cargo") : null);
  if(cargoHome && ["config","config.toml"].some(p=>existsSync(join(cargoHome,p)))) fail("external Cargo configuration is outside the measured scope");
}

export function measureCoverage({root, output, run=spawnSync, env=process.env}) {
  root=resolve(root); output=resolve(output);
  const rel=relative(root,output);
  if(!rel || (!rel.startsWith(`..${sep}`) && !isAbsolute(rel) && spawnSync("git",["check-ignore","-q",output],{cwd:root}).status !== 0)) fail("output inside checkout must be git-ignored");
  mkdirSync(dirname(output),{recursive:true});
  mkdirSync(output); // exclusive: never replace an earlier attempt
  const receipt={schema:"threads-rust-coverage/v1",qualified:false,startedAt:new Date().toISOString(),source:null,tools:{},command:null,coverage:null,files:[]};
  const save=()=>writeFileSync(join(output,"measurement.json"),json(receipt));
  const captureFiles=()=>{receipt.files=["lcov.info","llvm-summary.json","measurement.log","export.log","probes.json"].filter(p=>existsSync(join(output,p))).map(path=>{
    if(!lstatSync(join(output,path)).isFile()) fail("measurement artifact must be a regular file");
    const b=readFileSync(join(output,path));return {path,bytes:b.length,sha256:sha256(b)};
  });};
  save();
  try {
    rejectOverrides(root,env);
    if(process.versions.node !== "24.22.0") fail("Node 24.22.0 is required");
    receipt.source=snapshot(root);
    if(env.GITHUB_SHA && env.GITHUB_SHA !== receipt.source.commit) fail("GitHub checkout SHA differs from measured source");
    const probes=[];
    const probe=(command,args)=>{
      const r=run(command,args,{cwd:root,env,encoding:"utf8",maxBuffer:32*1024*1024});
      probes.push({command,args,status:r.status ?? null,signal:r.signal ?? null,stdout:r.stdout ?? "",stderr:r.stderr ?? "",error:r.error ? {code:r.error.code ?? null,message:r.error.message}:null});
      writeFileSync(join(output,"probes.json"),json(probes));
      if(r.error || r.signal || r.status !== 0) fail(`tool probe failed: ${command} ${args.join(" ")}`);
      return r.stdout.trim();
    };
    receipt.tools.node=process.version;
    receipt.tools.rustc=probe("rustc",["-vV"]);
    receipt.tools.cargo=probe("cargo",["--version"]);
    receipt.tools.cargoLlvmCov=probe("cargo",["llvm-cov","--version"]);
    if(!/^rustc 1\.88\.0\b/.test(receipt.tools.rustc) || !/^cargo 1\.88\.0\b/.test(receipt.tools.cargo) || receipt.tools.cargoLlvmCov !== "cargo-llvm-cov 0.9.1") fail("coverage tool versions differ from reviewed pins");
    const host=receipt.tools.rustc.match(/^host: (.+)$/m)?.[1];
    const llvmVersion=receipt.tools.rustc.match(/^LLVM version: (.+)$/m)?.[1];
    if(!host || !llvmVersion) fail("missing Rust host/LLVM version");
    const sysroot=probe("rustc",["--print","sysroot"]);
    receipt.tools.llvm={};
    for(const name of ["llvm-cov","llvm-profdata"]) {
      const binary=join(sysroot,"lib","rustlib",host,"bin",name + (process.platform === "win32" ? ".exe" : ""));
      const version=probe(binary,["--version"]);
      if(!version.includes(`LLVM version ${llvmVersion}`)) fail("LLVM tool version differs from rustc");
      receipt.tools.llvm[name]={version,sha256:sha256(readFileSync(binary))};
    }
    receipt.platform={os:process.platform,arch:process.arch,host,imageOS:env.ImageOS ?? null,imageVersion:env.ImageVersion ?? null};
    receipt.github={repository:env.GITHUB_REPOSITORY ?? null,runId:env.GITHUB_RUN_ID ?? null,attempt:env.GITHUB_RUN_ATTEMPT ?? null,event:env.GITHUB_EVENT_NAME ?? null,ref:env.GITHUB_REF ?? null};
    const args=["llvm-cov","--locked","--workspace","--lcov","--output-path",join(output,"lcov.info")];
    receipt.command={program:"cargo",args:[...args.slice(0,-1),"<output>/lcov.info"],status:null,signal:null};
    save();
    const fd=openSync(join(output,"measurement.log"),"wx");
    let result;
    try {result=run("cargo",args,{cwd:root,env,stdio:["ignore",fd,fd]});} finally {closeSync(fd);}
    receipt.command.status=result.status;
    receipt.command.signal=result.signal ?? null;
    receipt.command.error=result.error ? {code:result.error.code ?? null,message:result.error.message}:null;
    if(result.error || result.signal || result.status !== 0) fail("coverage command did not complete successfully");
    // FNF/FNH count LLVM function groups; FN/FNDA enumerate instantiations.
    // Export the same profile's summary to bind both denominators without guessing groups.
    const exportArgs=["llvm-cov","report","--locked","--workspace","--json","--summary-only","--output-path",join(output,"llvm-summary.json")];
    receipt.exportCommand={program:"cargo",args:[...exportArgs.slice(0,-1),"<output>/llvm-summary.json"],status:null,signal:null};
    const exportFd=openSync(join(output,"export.log"),"wx");
    let exported;
    try {exported=run("cargo",exportArgs,{cwd:root,env,stdio:["ignore",exportFd,exportFd]});} finally {closeSync(exportFd);}
    receipt.exportCommand.status=exported.status;
    receipt.exportCommand.signal=exported.signal ?? null;
    receipt.exportCommand.error=exported.error ? {code:exported.error.code ?? null,message:exported.error.message}:null;
    if(exported.error || exported.signal || exported.status !== 0) fail("LLVM summary export failed");
    if(JSON.stringify(snapshot(root)) !== JSON.stringify(receipt.source)) fail("coverage source changed during measurement");
    receipt.metadataCommand=["metadata","--locked","--offline","--format-version","1","--filter-platform",host];
    const metadata=JSON.parse(probe("cargo",receipt.metadataCommand));
    if(!metadata.resolve?.nodes?.length || !metadata.workspace_members?.length) fail("missing resolved feature inventory");
    const packages=new Map(metadata.packages.map(p=>[p.id,p]));
    if(packages.size !== metadata.packages.length || new Set(metadata.resolve.nodes.map(n=>n.id)).size !== metadata.resolve.nodes.length) fail("duplicate package resolution");
    receipt.resolvedFeatures=metadata.resolve.nodes.map(node=>{
      const p=packages.get(node.id), features=node.features;
      if(!p) fail("missing resolved package");
      if(!Array.isArray(features)) fail("missing package feature resolution");
      return {name:p.name,version:p.version,source:p.source,workspace:metadata.workspace_members.includes(p.id),features:[...features].sort()};
    }).sort((a,b)=>JSON.stringify(a).localeCompare(JSON.stringify(b),"en"));
    receipt.coverage=parseLcov(readFileSync(join(output,"lcov.info"),"utf8"),{root,sources:receipt.source.rustFiles,llvm:JSON.parse(readFileSync(join(output,"llvm-summary.json"),"utf8"))});
    receipt.scope={command:receipt.command.args,exportCommand:receipt.exportCommand.args,metadataCommand:receipt.metadataCommand,profile:"default test/dev",features:"workspace defaults; no extra features",doctests:false,defaultFilenameExclusions:true,tools:receipt.tools,platform:receipt.platform,config:receipt.source.config,sourcePaths:receipt.source.rustFiles.map(f=>f.path),reportedPaths:receipt.coverage.files.map(f=>f.path),resolvedFeatures:receipt.resolvedFeatures};
    receipt.scopeSha256=sha256(json(receipt.scope));
    if(JSON.stringify(snapshot(root)) !== JSON.stringify(receipt.source)) fail("coverage source changed during qualification");
    captureFiles();
    receipt.qualified=true;
  } catch(error) {
    receipt.qualified=false;
    receipt.error=error.message;
    try {captureFiles();} catch(indexError) {receipt.indexError=indexError.message;}
    receipt.finishedAt=new Date().toISOString(); save();
    throw error;
  }
  receipt.finishedAt=new Date().toISOString(); save();
  return receipt;
}

if(process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if(process.argv.length !== 3) {console.error("usage: node scripts/measure-rust-coverage.mjs NEW_OUTPUT_DIRECTORY");process.exitCode=2;}
  else {
    try {
      const r=measureCoverage({root:git(process.cwd(),["rev-parse","--show-toplevel"]),output:process.argv[2]});
      console.log(JSON.stringify({schema:r.schema,qualified:r.qualified,source:r.source.commit,tree:r.source.tree,scopeSha256:r.scopeSha256,tools:r.tools,platform:r.platform,resolvedFeaturesSha256:sha256(json(r.resolvedFeatures)),totals:r.coverage.totals,reportedFiles:r.coverage.files.length,unreportedFiles:r.coverage.unreported.length,files:r.files}));
    } catch(error) {console.error(`Coverage unqualified: ${error.message}`);process.exitCode=1;}
  }
}
