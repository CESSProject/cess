import * as path from "https://deno.land/std/path/mod.ts";
import { readStringDelim } from "https://deno.land/std/io/mod.ts";
import { copySync } from "https://deno.land/std/fs/copy.ts";
// import { sleep } from "https://deno.land/x/sleep/mod.ts";

async function startCeseal(basePath: string, port: string | number, tmpPath = "/tmp", extra_args = []) {
  const logPath = path.join(tmpPath, "ceseal.log");

  const args = [
      '--cores=0',  // Disable benchmark
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
      console.log(line);
      if (line.includes("Ceseal internal server will listening on")) {
        return true;
      }
    }
  }

  return true
}

const exists = async (filename: string): Promise<boolean> => {
  try {
    await Deno.stat(filename);
    // successful, file or directory must exist
    return true;
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      // file or directory does not exist
      return false;
    } else {
      // unexpected error, maybe permissions, pass it along
      throw error;
    }
  }
};

const currentPath = await Deno.realPath("/opt/ceseal/releases/current");
const currentVersion = currentPath.split("/").pop();
console.log(`Current ${currentPath}`)

// Check current (the image contains) has initialized
if (await exists(path.join(currentPath, "data/protected_files/runtime-data.seal"))) {
  console.log("runtime-data.seal exists, no need to handover")
  Deno.exit(0);
}

// TODO: descending sort folders and find the latest handoverable ceseal
let previousVersion: number | undefined = undefined;
for await (const dirEntry of Deno.readDir('/opt/ceseal/backups')) {
  // console.log(dirEntry);

  // TODO: check handoverable (initialized && synced && version). Q: how to deal with not synced?
  const version = parseInt(dirEntry.name);
  if (!previousVersion || previousVersion < version) {
    if (version >= currentVersion) {
      continue;
    } else if (!await exists(`/opt/ceseal/backups/${version}/data/protected_files/runtime-data.seal`)) {
      console.log(`no runtime-data.seal found in ${version}, skip`)
      continue
    }
    
    previousVersion = version;
  }
}

if (previousVersion === undefined) {
  console.log("No previous version, no need to handover!");

  // Copy current to backups
  try { copySync(currentPath, `/opt/ceseal/backups/${currentVersion}`) } catch (err) { console.error(err.message) }

  Deno.exit(0);
}

if (currentVersion == previousVersion) {
  console.log("same version, no need to handover")
  Deno.exit(0);
}

const previousPath = `/opt/ceseal/backups/${previousVersion}`;
console.log(`Previous ${previousPath}`);

console.log("starting");
try { Deno.removeSync("/tmp/ceseal.log") } catch (_err) {}
let oldProcess = await startCeseal(previousPath, "1888");
await waitCesealStarted("/tmp/ceseal.log");
console.log("started");

// Waiting old bin start, I'm thinking it's good to not get from api but just dump a file then pass to the new one?
// await sleep(30)

const command = new Deno.Command(`/opt/ceseal/releases/current/gramine-sgx`, {
  args: [
    "ceseal",
    "--request-handover-from=http://localhost:1888",
  ],
  cwd: "/opt/ceseal/releases/current"
});
const { code, stdout, stderr } = command.outputSync();

console.log(code);
console.log(new TextDecoder().decode(stdout));
console.log(new TextDecoder().decode(stderr));

// oldProcess.kill("SIGKILL");

if (code != 0) {
  console.log("Handover failed");
  Deno.exit(1);
}

console.log("Handover completed");

// Copy checkpoint from previous
const previousStoragePath = path.join(previousPath, "data/storage_files")
const storagePath = path.join(currentPath, "data/storage_files")
try { Deno.removeSync(storagePath) } catch (err) { console.error(err.message) }
try { copySync(previousStoragePath, storagePath) } catch (err) { console.error(err.message) }

// Copy current to backups
try { copySync(currentPath, `/opt/ceseal/backups/${currentVersion}`) } catch (err) { console.error(err.message) }

Deno.exit(0);
