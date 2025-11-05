use chrono::Local;
use clap::Parser;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, Signal, System};

/// 命令行参数解析
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// 要杀掉的进程名 (例如: electron-hotupdate-demo.exe)
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

/// 杀掉指定进程名的所有实例
fn kill_process_by_name(name: &str, logger: &Logger) {
    let mut sys = System::new_all();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );

    for (pid, process) in sys.processes() {
        let pname = process.name().to_string_lossy();
        if pname.eq_ignore_ascii_case(name) {
            logger.log(&format!("Killing process {:?} (pid {})", pname, pid));
            let _ = process.kill_with(Signal::Kill);
        }
    }
}

/// 复制文件（保留目录结构），同名文件覆盖，不清空目标目录
fn copy_dir_recursive(input: &Path, output: &Path, logger: &Logger) -> io::Result<()> {
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
        let dest = output.join(relative);

        if path.is_dir() {
            fs::create_dir_all(&dest)?;
            copy_dir_recursive(&path, &dest, logger)?;
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

    // 初始化日志器
    let logger = Logger::new(args.log.as_deref()).unwrap_or_else(|e| {
        eprintln!("Failed to initialize logger: {}", e);
        std::process::exit(1);
    });

    logger.log("Updater started");
    logger.log(&format!("App path: {}", args.app));
    logger.log(&format!("Process name: {}", args.ps));
    logger.log(&format!("Input dir: {}", args.input));
    logger.log(&format!("Output dir: {}", args.output));

    let ps_name = Path::new(&args.ps)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    // 杀掉主程序
    kill_process_by_name(ps_name, &logger);

    // 执行文件复制
    let input_path = PathBuf::from(&args.input);
    let output_path = PathBuf::from(&args.output);

    if let Err(e) = copy_dir_recursive(&input_path, &output_path, &logger) {
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
