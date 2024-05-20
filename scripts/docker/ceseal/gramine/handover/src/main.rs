mod arg;
mod error;
use arg::Args;
use clap::Parser;
use core::panic;
use error::Error;
use std::{path::Path, process::Stdio};
use tokio::{
	io::{AsyncBufReadExt, BufReader, BufWriter},
	process::{Child, ChildStdout, Command},
};

#[tokio::main]
async fn main() {
	let args = Args::parse();

	let mut current_version: u64 = 0;
	let mut previous_version: u64 = 0;
	match tokio::fs::read_link(&args.current_version_ceseal_path).await {
		Ok(real_path) =>
			if let Some(path_str) = real_path.to_str() {
				current_version = path_str
					.split("/")
					.last()
					.expect("no last version number")
					.parse::<u64>()
					.expect("parse current ceseal version from str to u64 failed!");
			} else {
				panic!("can't get real path of current ceseal");
			},
		Err(e) => panic!("Error reading symlink {}: {}", args.current_version_ceseal_path, e),
	}
	log(format!("Current version: {}", current_version));

	// Get the path to the current Ceseal version and check whether it has been initialized.
	let current_ceseal_runtime_data_path =
		Path::new(&args.current_version_ceseal_path).join(&args.ceseal_runtime_data_seal_path);
	if current_ceseal_runtime_data_path.exists() {
		log(format!("runtime-data.seal exists, no need to handover"));
		return
	}

	let current_ceseal_backup_path = Path::new(&args.previous_version_ceseal_path);
	match confirm_previous_ceseal_version(
		args.previous_version_ceseal_path.clone(),
		args.ceseal_runtime_data_seal_path.clone(),
		current_version,
	) {
		Ok(ver) => {
			// Otherwise, confirm whether a previous version exists. If there is no previous version, no handover is
			// required, back up the current version to the backup directory and exit.
			if ver == 0 {
				log(format!("No previous version, no need to handover!"));

				let current_ceseal_backup_path = current_ceseal_backup_path.join(current_version.to_string());
				if let Err(err) = tokio::fs::copy(args.current_version_ceseal_path, current_ceseal_backup_path).await {
					panic!("Error backing up current version: {}", err);
				}
				return
			}
			//If the current version is the same as the previous version, there is no need to hand over and exit
			// directly.
			if ver == current_version {
				log(format!("same version, no need to handover"));
				return
			}
			previous_version = ver;
		},
		Err(e) => {
			panic!("confirm_previous_ceseal_version error :{:?}", e);
		},
	};

	let previous_ceseal_path = Path::new(&args.previous_version_ceseal_path).join(previous_version.to_string());
	log(format!("Previous ${previous_version}"));
	let current_ceseal_storage_path =
		Path::new(&args.current_version_ceseal_path).join(&args.ceseal_storage_files_path);
	let previous_ceseal_storage_path = previous_ceseal_path.join(&args.ceseal_storage_files_path);

	tokio::fs::remove_file(&args.ceseal_log_path)
		.await
		.expect("remove old ceseal log file fail");

	//start old ceseal
	let mut old_process = start_previous_ceseal(previous_ceseal_path.to_str().unwrap().to_string(), args.previous_ceseal_port.clone())
		.await
		.expect("start previous ceseal fail");
	redirect_ceseal_runtime_log(old_process.stdout.take().expect("previous ceseal log output in invaid!"), args.ceseal_log_path.clone())
		.await
		.expect("redirect ceseal runtime log fail");

	//wait for old ceseal went well
	wait_for_ceseal_to_run_successfully(args.ceseal_log_path.clone())
		.await
		.expect("wait for ceseal log fail");
	log(format!("previous ceseal started!"));

	ensure_data_dir(current_ceseal_storage_path.parent().unwrap().to_str().unwrap())
		.expect("ensure current data dir fail");


	//start current version ceseal
	let command = Command::new("/opt/ceseal/releases/current/gramine-sgx")
        .args(&["ceseal", &format!("--request-handover-from=http://localhost:{:?}",args.previous_ceseal_port.clone())])
        .current_dir(&args.current_version_ceseal_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
	match command {
        Ok(child) => {
            let output = child.wait_with_output().await.expect("Failed to wait for command to execute");
            let code = output.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            log(format!("Exit code: {}", code));
			log(format!("Stdout:\n{}", stdout));
			log(format!("Stderr:\n{}", stderr));

            if code != 0 {
                log("Handover failed".to_string());
                return
            }
        },
        Err(e) => panic!("Error executing current ceseal command: {}", e),
    }
	old_process.kill().await.expect("old ceseal stop fail");

	match tokio::fs::remove_dir_all(&current_ceseal_storage_path).await {
        Ok(_) => log("Removed previous storage successfully.".to_string()),
        Err(e) => eprintln!("Error removing previous storage: {}", e),
    }
    match tokio::fs::copy(&previous_ceseal_storage_path, &current_ceseal_storage_path).await {
        Ok(_) => log("Copied checkpoint from previous successfully.".to_string()),
        Err(e) => eprintln!("Error copying checkpoint from previous: {}", e),
    }

    // 备份当前版本到备份目录
    match tokio::fs::copy(&args.current_version_ceseal_path, &current_ceseal_backup_path).await {
        Ok(_) => log(format!("Backed up current version successfully to {:?}", current_ceseal_backup_path)),
        Err(e) => eprintln!("Error backing up current version: {}", e),
    }

}

pub async fn start_previous_ceseal(previous_ceseal_path: String, port: String) -> Result<Child, Error> {
	let extra_args: &[&str] = &vec!["--port", &port];

	let mut cmd = Command::new(Path::new(&previous_ceseal_path).join("start.sh"));
	cmd.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		// .stderr(Stdio::piped())
		.env("SKIP_AESMD", "1")
		.env("EXTRA_OPTS", extra_args.join(" "));

	let child = cmd.spawn().map_err(|e| Error::StartCesealFailed(e.to_string()))?;

	Ok(child)
}

pub async fn redirect_ceseal_runtime_log(stdout: ChildStdout, log_path: String) -> Result<(), Error> {
	//redirect process log into new created log file
	let log_path = Path::new(&log_path);
	let log_file = tokio::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.open(log_path)
		.await
		.map_err(|e| Error::RedirectCesealLogFailed(e.to_string()))?;
	let mut log_writer = BufWriter::new(log_file);

	let mut reader = BufReader::new(stdout);

	tokio::spawn(async move {
		tokio::io::copy(&mut reader, &mut log_writer)
			.await
			.expect("Error piping stdout to log file");
	});
	Ok(())
}

pub async fn wait_for_ceseal_to_run_successfully(log_path: String) -> Result<(), Error> {
	let log_file = tokio::fs::File::open(log_path)
		.await
		.map_err(|e| Error::DetectCesealRunningStatueFailed(e.to_string()))?;
	let mut reader = BufReader::new(log_file);
	let mut line = String::new();
	loop {
		match reader.read_line(&mut line).await {
			Ok(bytes_read) if bytes_read > 0 => {
				log(format!("ceseal log is :{}", line));

				if line.contains("Ceseal internal server will listening on") {
					return Ok(())
				}
				line.clear();
			},
			Ok(_) => break,
			Err(err) => return Err(Error::DetectCesealRunningStatueFailed(err.to_string())),
		}
	}
	Err(Error::DetectCesealRunningStatueFailed("no log of normal startup of ceseal was detected".to_string()))
}

pub fn confirm_previous_ceseal_version(
	previous_version_ceseal_path: String,
	ceseal_runtime_data_seal_path: String,
	current_version: u64,
) -> Result<u64, Error> {
	let entries =
		std::fs::read_dir(&previous_version_ceseal_path).map_err(|e| Error::PreviousVersionFailed(e.to_string()))?;

	let mut versiont_list: Vec<u64> = Vec::new();
	for entry in entries {
		if let Ok(entry) = entry {
			let path = entry.path();
			if path.is_file() {
				if let Some(file_name) = path.file_name() {
					versiont_list.push(
						file_name
							.to_str()
							.ok_or(Error::PreviousVersionFailed(format!(
								"error file appears in the path {:?}",
								&previous_version_ceseal_path
							)))?
							.to_string()
							.parse::<u64>()
							.map_err(|e| Error::PreviousVersionFailed(e.to_string()))?,
					)
				}
			}
		}
	}
	versiont_list.sort_by(|a, b| b.cmp(a));

	let mut previous_version = 0;
	for version in versiont_list {
		if previous_version == 0 || previous_version < version {
			if version >= current_version {
				continue
			} else if !Path::new(&previous_version_ceseal_path)
				.join(&version.to_string())
				.join(&ceseal_runtime_data_seal_path)
				.exists()
			{
				log(format!("no runtime-data.seal found in ${version}, skip"));
				continue
			}
			previous_version = version;
			break
		}
	}

	Ok(previous_version)
}

fn ensure_data_dir(data_dir: &str) -> Result<(), std::io::Error> {
	if !Path::new(data_dir).exists() {
		std::fs::create_dir_all(data_dir)?;
	}

	// Create the protected_files subdirectory if it does not exist
	let protected_files_dir = Path::new(data_dir).join("protected_files");
	if !protected_files_dir.exists() {
		std::fs::create_dir_all(&protected_files_dir)?;
	}

	// Create the storage_files subdirectory if it does not exist
	let storage_files_dir = Path::new(data_dir).join("storage_files");
	if !storage_files_dir.exists() {
		std::fs::create_dir_all(&storage_files_dir)?;
	}

	Ok(())
}

const LOG_PREFIX: &str = "[Handover🤝]";
fn log(log_text: String) {
	println!("{:?} {:?}", LOG_PREFIX, log_text)
}
