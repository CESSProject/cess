import * as path from "https://deno.land/std/path/mod.ts";
import { readStringDelim } from "https://deno.land/std/io/mod.ts";
import { copySync } from "https://deno.land/std/fs/copy.ts";
import { sortBy } from "https://deno.land/std@0.214.0/collections/sort_by.ts";
import { exists } from "https://deno.land/std@0.214.0/fs/mod.ts";
// import { sleep } from "https://deno.land/x/sleep/mod.ts";

const LOG_PREFIX = "[Handover🤝]"

function log(...args: any[]) {
  args.unshift(LOG_PREFIX + " ");
  console.log(...args);
}

async function startCeseal(basePath: string, port: string | number, tmpPath = "/tmp", extra_args = []) {
  const logPath = path.join(tmpPath, "ceseal.log");

  const args = [
    '--port', port.toString(),
  ];
  args.push(...extra_args);

  const bin = new Deno.Command(`${basePath}/start.sh`, {
    stdin: "piped",
    stdout: "piped",
    // stderr: "piped",
    env: {
      "SKIP_AESMD": "1",
      "EXTRA_OPTS": args.join(" ")
    }
  });
  const child = bin.spawn();

  child.stdout.pipeTo(Deno.openSync(logPath, { read: true, write: true, create: true }).writable);

  return child;
}

async function waitCesealStarted(logFile: string) {
  const fileReader = await Deno.open(logFile, { read: true, write: true, create: true });

  const watcher = Deno.watchFs(logFile);
  for await (const event of watcher) {
    if (event.kind !== "modify") continue;
    for await (const line of readStringDelim(fileReader, "\n")) {
      if (!line) break;
      log(line);
      if (line.includes("Ceseal internal server will listening on")) {
        return true;
      }
    }
  }

  return true
}

async function confirmPreviousVersion(): Promise<number | undefined> {
  let versions: number[] = [];
  for await (const dirEntry of Deno.readDir('/opt/ceseal/backups')) {
    versions.push(parseInt(dirEntry.name));
  }
  const sortedByDesc = sortBy(versions, (it) => it, { order: "desc" });
  let previousVersion: number | undefined = undefined;
  for (const version of sortedByDesc) {
    if (!previousVersion || previousVersion < version) {
      if (version >= currentVersion) {
        continue;
      } else if (!await exists(`/opt/ceseal/backups/${version}/data/protected_files/runtime-data.seal`)) {
        console.log(`no runtime-data.seal found in ${version}, skip`)
        continue
      }
      previousVersion = version;
      break;
    }
  }
  return previousVersion;
}

async function killPreviousCeseal(version: number) {
  const cmd = ["bash", "-c", `ps -eaf | grep "backups/${version}/cruntime/sgx/loader" | grep -v "grep" | awk '{print $2}'`];
  const p = Deno.run({ cmd, stdout: "piped", stderr: "piped" });

  // Reading the outputs closes their pipes
  const [{ code }, rawOutput, rawError] = await Promise.all([
    p.status(),
    p.output(),
    p.stderrOutput(),
  ]);

  if (code === 0) {
    const pid = new TextDecoder().decode(rawOutput);
    log(`the previous version ${version} ceseal pid: ${pid}`);
    const p = Deno.run({ cmd: ["bash", "-c", `kill -9 ${pid}`] });
    await p.status();
  } else {
    const errorString = new TextDecoder().decode(rawError);
    log(errorString);
  }
}

function ensureDataDir(dataDir: string) {
  try {
    const fileInfo = Deno.lstatSync(dataDir);
    if (fileInfo.isSymlink) {
      const target = Deno.readLinkSync(dataDir);
      Deno.mkdirSync(target, { recursive: true });
    }
  } catch (err) {
    if (err.name === "NotFound") {
      Deno.mkdirSync(dataDir, { recursive: true });
    } else {
      throw err;
    }
  }
  try { Deno.mkdirSync(path.join(dataDir, "protected_files"), { recursive: true }) } catch (err) { console.log(err) }
  try { Deno.mkdirSync(path.join(dataDir, "storage_files"), { recursive: true }) } catch (err) { console.log(err) }
}

const currentPath = await Deno.realPath("/opt/ceseal/releases/current");
const currentVersion = currentPath.split("/").pop();
log(`Current ${currentPath}`)

// Check current (the image contains) has initialized
if (await exists(path.join(currentPath, "data/protected_files/runtime-data.seal"))) {
  log("runtime-data.seal exists, no need to handover")
  Deno.exit(0);
}

let previousVersion: number | undefined = await confirmPreviousVersion();

if (previousVersion === undefined) {
  log("No previous version, no need to handover!");

  // Copy current to backups
  try { copySync(currentPath, `/opt/ceseal/backups/${currentVersion}`) } catch (err) { console.error(err.message) }

  Deno.exit(0);
}

if (currentVersion == previousVersion) {
  log("same version, no need to handover")
  Deno.exit(0);
}

const previousPath = `/opt/ceseal/backups/${previousVersion}`;
log(`Previous ${previousPath}`);

const previousStoragePath = path.join(previousPath, "data/storage_files");
const currentDataDir = path.join(currentPath, "data");
const currentStoragePath = path.join(currentDataDir, "storage_files");

log("starting");
try { Deno.removeSync("/tmp/ceseal.log") } catch (_err) { }
if (previousVersion == 24013112) {
  log(`skip the storage files of version ${previousVersion}`);
  const cmd = ["bash", "-c", `rm -rf ${previousStoragePath}/*`];
  const p = Deno.run({ cmd });
  await p.status();
}
let oldProcess = await startCeseal(previousPath, "1888");
try {
  await waitCesealStarted("/tmp/ceseal.log");
  log("started");

  // Waiting old bin start, I'm thinking it's good to not get from api but just dump a file then pass to the new one?
  // await sleep(30)  

  ensureDataDir(currentDataDir);

  const command = new Deno.Command(`/opt/ceseal/releases/current/gramine-sgx`, {
    args: [
      "ceseal",
      "--request-handover-from=http://localhost:1888",
    ],
    cwd: "/opt/ceseal/releases/current"
  });
  const { code, stdout, stderr } = command.outputSync();

  log(code);
  log(new TextDecoder().decode(stdout));
  log(new TextDecoder().decode(stderr));

  if (code != 0) {
    log("Handover failed");
    Deno.exit(1);
  }

} finally {
  oldProcess.kill();  //the kill() method not to kill the ceseal process due to it's running use exec
  await killPreviousCeseal(previousVersion as number);
}

log("Handover completed");

// Copy checkpoint from previous
try { Deno.removeSync(currentStoragePath) } catch (err) { console.error(err.message) }
try { copySync(previousStoragePath, currentStoragePath) } catch (err) { console.error(err.message) }

// Copy current to backups
try { copySync(currentPath, `/opt/ceseal/backups/${currentVersion}`) } catch (err) { console.error(err.message) }

Deno.exit(0);
