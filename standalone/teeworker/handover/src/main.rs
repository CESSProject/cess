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
	let current_ceseal_runtime_data_path = Path::new(&args.current_version_ceseal_path)
		.join(&args.ceseal_protected_files_path)
		.join("runtime-data.seal");
	if current_ceseal_runtime_data_path.exists() {
		log(format!("runtime-data.seal exists, no need to handover"));
		return
	}

	let current_ceseal_backup_path = Path::new(&args.previous_version_ceseal_path).join(current_version.to_string());
	match confirm_previous_ceseal_version(
		args.previous_version_ceseal_path.clone(),
		args.ceseal_protected_files_path.clone(),
		args.ceseal_data_path.clone(),
		current_version,
	)
	.await
	{
		Ok(ver) => {
			// Anyway, back up the current version
			if !current_ceseal_backup_path.exists() {
				if let Err(err) =
					copy_directory(Path::new(&args.current_version_ceseal_path), &current_ceseal_backup_path).await
				{
					panic!("Error backing up current version: {}", err);
				}
			}

			if ver == 0 {
				log(format!("No previous version, no need to handover!"));
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

	if let Err(err) = tokio::fs::remove_file(&args.previous_ceseal_log_path).await {
		log(format!("remove old ceseal log file fail : {err}"))
	};
	if let Err(err) = tokio::fs::remove_file(&args.new_ceseal_log_path).await {
		log(format!("remove new ceseal log file fail : {err}"))
	};

	//start old ceseal
	let mut old_process = start_previous_ceseal(
		previous_ceseal_path.to_str().unwrap().to_string(),
		args.previous_ceseal_port.to_string(),
	)
	.await
	.expect("start previous ceseal fail");
	redirect_ceseal_runtime_log(
		old_process.stdout.take().expect("previous ceseal log output in invaid!"),
		args.previous_ceseal_log_path.clone(),
	)
	.await
	.expect("redirect ceseal runtime log fail");

	//wait for preivious ceseal went well
	wait_for_ceseal_to_run_successfully(
		args.previous_ceseal_log_path.clone(),
		"Ceseal internal server will listening on",
	)
	.await
	.expect("wait for previous ceseal log fail");
	log(format!("previous ceseal started!"));

	let current_ceseal_real_storage_path =
		Path::new(&args.ceseal_data_path).join(&current_version.to_string());
	ensure_data_dir(&current_ceseal_real_storage_path)
		.await
		.expect("ensure current data dir fail");

	//start current version ceseal
	let command = Command::new("/opt/ceseal/releases/current/gramine-sgx")
		.args(&["ceseal", &format!("--request-handover-from=http://localhost:{}", args.previous_ceseal_port)])
		.current_dir(&args.current_version_ceseal_path)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn();
	match command {
		Ok(mut child) => {
			redirect_ceseal_runtime_log(
				child.stdout.take().expect("new ceseal log output in invaid!"),
				args.new_ceseal_log_path.clone(),
			)
			.await
			.expect("redirect ceseal runtime log fail");
			wait_for_ceseal_to_run_successfully(args.new_ceseal_log_path, "Handover done")
				.await
				.expect("wait for new ceseal log fail");
			log(format!("handover success!"));
		},
		Err(e) => panic!("Error executing current ceseal command: {}", e),
	}
	old_process.kill().await.expect("old ceseal stop fail");
	kill_previous_ceseal(previous_version).await;

	let current_ceseal_storage_path =
		Path::new(&args.current_version_ceseal_path).join(&args.ceseal_storage_files_path);
	let previous_ceseal_storage_path = previous_ceseal_path.join(&args.ceseal_storage_files_path);
	match tokio::fs::remove_dir_all(&current_ceseal_storage_path).await {
		Ok(_) => log("Removed current storage successfully.".to_string()),
		Err(e) => eprintln!("Error removing previous storage: {}", e),
	}
	match copy_directory(&previous_ceseal_storage_path, &current_ceseal_storage_path).await {
		Ok(_) => log("Copied checkpoint from previous successfully.".to_string()),
		Err(e) => panic!("Error copying checkpoint from previous: {}", e),
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

pub async fn wait_for_ceseal_to_run_successfully(log_path: String, flag: &str) -> Result<(), Error> {
	let sleep_for_ceseal_running = tokio::time::Duration::from_secs(5);
	let mut sleep_times = 3;
	let log_file = tokio::fs::File::open(&log_path)
		.await
		.map_err(|e| Error::DetectCesealRunningStatueFailed(e.to_string()))?;
	let mut reader = BufReader::new(log_file);
	let mut line = String::new();
	loop {
		match reader.read_line(&mut line).await {
			Ok(bytes_read) if bytes_read > 0 => {
				log(format!("{}:{}", &log_path, line));

				if line.contains(flag) {
					return Ok(())
				}
				line.clear();
			},
			Ok(_) => {
				if sleep_times > 0 {
					tokio::time::sleep(sleep_for_ceseal_running).await;
					sleep_times -= 1;
					continue
				}
				return Err(Error::DetectCesealRunningStatueFailed("ceseal log has no content".to_string()))
			},
			Err(err) => return Err(Error::DetectCesealRunningStatueFailed(err.to_string())),
		}
	}
}

pub async fn confirm_previous_ceseal_version(
	previous_version_ceseal_path: String,
	ceseal_protected_files_path: String,
	ceseal_data_path: String,
	current_version: u64,
) -> Result<u64, Error> {
	if !Path::new(&previous_version_ceseal_path).exists() {
		tokio::fs::create_dir_all(&previous_version_ceseal_path)
			.await
			.map_err(|e| Error::PreviousVersionFailed(e.to_string()))?;
	}
	let mut entries = tokio::fs::read_dir(&previous_version_ceseal_path)
		.await
		.map_err(|e| Error::PreviousVersionFailed(e.to_string()))?;
	let mut versiont_list: Vec<u64> = Vec::new();
	while let Some(entry) = entries
		.next_entry()
		.await
		.map_err(|e| Error::PreviousVersionFailed(e.to_string()))?
	{
		let path = entry.path();
		if path.is_dir() {
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
	versiont_list.sort_by(|a, b| b.cmp(a));

	let mut previous_version = 0;
	for version in versiont_list {
		if previous_version == 0 || previous_version < version {
			if version >= current_version {
				continue
			}
			previous_version = version;
			break
		}
	}
	//Everytime publish next version of ceseal will be detect overhere
	//Let's backup the previous runtime-data.seal file into 'backups ceseal's data' first
	if previous_version != 0 {
		let previous_runtime_data = Path::new(&previous_version_ceseal_path)
			.join(&previous_version.to_string())
			.join(&ceseal_protected_files_path)
			.join("runtime-data.seal");
		if !previous_runtime_data.exists() {
			log(format!("no runtime-data.seal found in version {previous_version}, sync from {ceseal_data_path} now"));
			//Since the runtimedata generated by ceseal was not saved to the '/opt/ceseal/backups' path after exiting
			// when it was started for the first time, it needs to be synchronized here to avoid data loss from data to
			// backup.
			copy_directory(
				&Path::new(&ceseal_data_path).join(&previous_version.to_string()),
				&Path::new(&previous_version_ceseal_path)
					.join(&previous_version.to_string())
					.join(
						&ceseal_protected_files_path
							.split("/")
							.next()
							.unwrap()
							.parse::<String>()
							.unwrap(),
					),
			)
			.await?;
		}
	}

	Ok(previous_version)
}

async fn ensure_data_dir(data_dir: &Path) -> Result<(), std::io::Error> {
	if !data_dir.exists() {
		tokio::fs::create_dir_all(data_dir).await?;
	}

	// Create the protected_files subdirectory if it does not exist
	let protected_files_dir = data_dir.join("protected_files");
	if !protected_files_dir.exists() {
		tokio::fs::create_dir_all(&protected_files_dir).await?;
		log("create protected file for current ceseal...".to_string())
	}

	let storage_files_dir = data_dir.join("storage_files");
	if !storage_files_dir.exists() {
		tokio::fs::create_dir_all(&storage_files_dir).await?;
		log("create storage file for current ceseal...".to_string())
	}
	Ok(())
}

const LOG_PREFIX: &str = "[Handover🤝]";
fn log(log_text: String) {
	println!("{} {}", LOG_PREFIX, log_text)
}

use walkdir::WalkDir;
async fn copy_directory(source: &Path, destination: &Path) -> Result<(), Error> {
	let mut tasks = vec![];
	for entry in WalkDir::new(source).into_iter().filter_map(Result::ok) {
		let path = entry.path();
		let relative_path = path.strip_prefix(source).map_err(|e| Error::CopyDirectory(e.to_string()))?;
		let dest_path = destination.join(relative_path);

		if path.is_dir() {
			tokio::fs::create_dir_all(&dest_path)
				.await
				.map_err(|e| Error::CopyDirectory(e.to_string()))?;
		} else if path.is_file() {
			let path = path.to_path_buf();
			let dest_path = dest_path.to_path_buf();
			let task = tokio::spawn(async move { tokio::fs::copy(&path, &dest_path).await });
			tasks.push(task);
		}
	}

	for task in tasks {
		let _ = task.await;
	}
	Ok(())
}

pub async fn kill_previous_ceseal(version: u64) {
	let cmd =
		format!("ps -eaf | grep \"backups/{}/cruntime/sgx/loader\" | grep -v \"grep\" | awk '{{print $2}}'", version);

	let process = Command::new("bash")
		.arg("-c")
		.arg(cmd)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("Failed to spawn process");

	let output = process.wait_with_output().await.expect("Failed to read output");

	if output.status.success() {
		let pid_str = std::str::from_utf8(&output.stdout).expect("Failed to parse output as UTF-8");
		if let Ok(pid) = pid_str.trim().parse::<i32>() {
			log(format!("kill the previous version {} ceseal pid: {}", version, pid));
			Command::new("kill")
				.arg("-9")
				.arg(pid.to_string())
				.status()
				.await
				.expect("Failed to kill process");
		}
	} else {
		let error_str = std::str::from_utf8(&output.stderr).expect("Failed to parse error as UTF-8");
		log(format!("{}", error_str));
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use tokio::test;
	#[test]
	async fn start_old_ceseal() {
		let args = Args::parse();
		let previous_version = 1;
		let previous_ceseal_path = Path::new(&args.previous_version_ceseal_path).join(previous_version.to_string());
		log(format!("Previous ${previous_version}"));

		if let Err(err) = tokio::fs::remove_file(&args.previous_ceseal_log_path).await {
			log(format!("remove old ceseal log file fail : {err}"))
		};

		//start old ceseal
		let mut old_process = start_previous_ceseal(
			previous_ceseal_path.to_str().unwrap().to_string(),
			args.previous_ceseal_port.to_string(),
		)
		.await
		.expect("start previous ceseal fail");
		redirect_ceseal_runtime_log(
			old_process.stdout.take().expect("previous ceseal log output in invaid!"),
			args.previous_ceseal_log_path.clone(),
		)
		.await
		.expect("redirect ceseal runtime log fail");

		//wait for old ceseal went well
		wait_for_ceseal_to_run_successfully(
			args.previous_ceseal_log_path.clone(),
			"Ceseal internal server will listening on",
		)
		.await
		.expect("wait for ceseal log fail");
		log(format!("previous ceseal started!"));
	}
}
