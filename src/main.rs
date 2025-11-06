use chrono::Local;
use clap::Parser;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, Signal, System};

/// 命令行参数解析
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// 要杀掉的进程名 (例如: yourApp.exe,otherApp.exe)
    #[arg(long)]
    ps: String,

    /// 更新输入目录 (更新文件所在目录)
    #[arg(long)]
    input: String,

    /// 输出目录 (一般为 app 的 resources 目录)
    #[arg(long)]
    output: String,

    /// Electron 应用主程序路径
    #[arg(long)]
    app: String,

    /// 日志文件路径（可选），默认在当前 exe 同级目录
    #[arg(long)]
    log: Option<String>,

    /// 要忽略复制的文件/目录（以逗号分隔，路径相对于 input）
    #[arg(long)]
    ignore: Option<String>,
}

/// 日志器结构体
struct Logger {
    file: Option<Arc<Mutex<File>>>,
}

impl Logger {
    fn new(log_path: Option<&str>) -> io::Result<Self> {
        let file = if let Some(path) = log_path {
            Some(Arc::new(Mutex::new(
                OpenOptions::new().create(true).append(true).open(path)?,
            )))
        } else {
            // 默认路径：当前 exe 同级目录 / updater.log
            let exe = std::env::current_exe()?;
            let default_path = exe
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("updater.log");
            Some(Arc::new(Mutex::new(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(default_path)?,
            )))
        };
        Ok(Self { file })
    }

    fn log(&self, msg: &str) {
        let now = Local::now();
        let line = format!("[{}] {}\n", now.format("%Y-%m-%d %H:%M:%S"), msg);
        print!("{}", line);

        if let Some(f) = &self.file {
            let mut f = f.lock().unwrap();
            let _ = f.write_all(line.as_bytes());
        }
    }
}

/// 杀掉多个指定进程名的所有实例（支持逗号分隔），并等待退出确认
fn kill_processes_by_names(names: &str, logger: &Logger) {
    let targets: Vec<String> = names
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if targets.is_empty() {
        logger.log("No process names provided, skipping kill step.");
        return;
    }

    let mut sys = System::new_all();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );

    // 先发送 Kill 信号
    for (pid, process) in sys.processes() {
        let pname = process.name().to_string_lossy().to_string();
        if targets.iter().any(|t| pname.eq_ignore_ascii_case(t)) {
            logger.log(&format!("Killing process {:?} (pid {})", pname, pid));
            if process.kill_with(Signal::Kill).is_none() {
                logger.log(&format!("Failed to send kill signal to {:?}", pname));
            }
        }
    }

    // 再等待确认退出
    const MAX_WAIT_MS: u64 = 5000; // 最多等待 5 秒
    const CHECK_INTERVAL_MS: u64 = 500;

    let mut elapsed = 0;
    loop {
        thread::sleep(Duration::from_millis(CHECK_INTERVAL_MS));
        elapsed += CHECK_INTERVAL_MS;

        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::everything(),
        );

        let alive: Vec<_> = sys
            .processes()
            .values()
            .filter(|p| {
                let pname = p.name().to_string_lossy();
                targets.iter().any(|t| pname.eq_ignore_ascii_case(t))
            })
            .map(|p| p.name().to_string_lossy().to_string())
            .collect();

        if alive.is_empty() {
            logger.log("All target processes have exited.");
            break;
        } else {
            logger.log(&format!("Waiting for processes to exit: {:?}", alive));
        }

        if elapsed >= MAX_WAIT_MS {
            logger.log("Timeout waiting for processes to exit, continue anyway.");
            break;
        }
    }
}

/// 复制文件（保留目录结构），同名文件覆盖，不清空目标目录
fn copy_dir_recursive(
    input: &Path,
    output: &Path,
    ignores: &[String],
    logger: &Logger,
) -> io::Result<()> {
    if !input.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Input directory not found",
        ));
    }

    for entry in fs::read_dir(input)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(input).unwrap();
        let relative_str = relative.to_string_lossy().replace('\\', "/"); // ✅ 统一路径分隔符
        let dest = output.join(relative);

        // ✅ 检查是否在忽略列表中
        if ignores
            .iter()
            .any(|ignore| relative_str.starts_with(ignore))
        {
            logger.log(&format!("Ignored: {}", relative_str));
            continue;
        }

        if path.is_dir() {
            fs::create_dir_all(&dest)?;
            copy_dir_recursive(&path, &dest, ignores, logger)?;
        } else {
            fs::create_dir_all(dest.parent().unwrap())?;
            fs::copy(&path, &dest)?;
            logger.log(&format!("Copied file: {}", dest.display()));
        }
    }

    Ok(())
}

fn main() {
    let args = Args::parse();

    let logger = Logger::new(args.log.as_deref()).unwrap_or_else(|e| {
        eprintln!("Failed to initialize logger: {}", e);
        std::process::exit(1);
    });

    logger.log("Updater started");
    logger.log(&format!("App path: {}", args.app));
    logger.log(&format!("Process name(s): {}", args.ps));
    logger.log(&format!("Input dir: {}", args.input));
    logger.log(&format!("Output dir: {}", args.output));

    // ✅ 解析忽略路径
    let ignores: Vec<String> = args
        .ignore
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().replace('\\', "/")) // 统一路径分隔符
        .filter(|s| !s.is_empty())
        .collect();

    if !ignores.is_empty() {
        logger.log(&format!("Ignore list: {:?}", ignores));
    }

    kill_processes_by_names(&args.ps, &logger);

    // 执行文件复制
    let input_path = PathBuf::from(&args.input);
    let output_path = PathBuf::from(&args.output);

    if let Err(e) = copy_dir_recursive(&input_path, &output_path, &ignores, &logger) {
        logger.log(&format!("File copy failed: {}", e));
        return;
    }

    logger.log("File copy completed successfully");

    // 重启主程序
    if Path::new(&args.app).exists() {
        logger.log("Restarting main app...");
        let _ = Command::new(&args.app)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        logger.log("Main app restarted");
    } else {
        logger.log("Main app not found, skip restart");
    }

    logger.log("Updater finished");
}
