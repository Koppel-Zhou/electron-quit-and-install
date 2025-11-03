use std::{
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, exit},
};
use sysinfo::{ProcessRefreshKind, RefreshKind, Signal, System};

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut app = None;
    let mut ps = None;
    let mut input = None;
    let mut output = None;

    for arg in &args[1..] {
        if let Some(v) = arg.strip_prefix("--app=") {
            app = Some(v.to_string());
        } else if let Some(v) = arg.strip_prefix("--ps=") {
            ps = Some(v.to_string());
        } else if let Some(v) = arg.strip_prefix("--input=") {
            input = Some(v.to_string());
        } else if let Some(v) = arg.strip_prefix("--output=") {
            output = Some(v.to_string());
        }
    }

    let app = app.unwrap_or_else(|| {
        eprintln!("missing --app");
        exit(1);
    });
    let ps = ps.unwrap_or_else(|| {
        eprintln!("missing --ps");
        exit(1);
    });
    let input = PathBuf::from(input.unwrap_or_else(|| {
        eprintln!("missing --input");
        exit(1);
    }));
    let output = PathBuf::from(output.unwrap_or_else(|| {
        eprintln!("missing --output");
        exit(1);
    }));

    println!("Killing processes: {}", ps);
    kill_processes(&ps);

    println!("Copying from {:?} to {:?}", input, output);
    if let Err(e) = copy_dir_all(&input, &output) {
        eprintln!("copy error: {}", e);
        exit(1);
    }

    println!("Starting app: {}", app);
    if let Err(e) = Command::new(&app).spawn() {
        eprintln!("failed to start app: {}", e);
        exit(1);
    }

    println!("Done.");
}

fn kill_processes(ps_names: &str) {
    let targets: Vec<String> = ps_names
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    // 初始化系统信息（只刷新进程信息）
    let mut sys = System::new_with_specifics(
        RefreshKind::everything().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_all();

    for (pid, process) in sys.processes() {
        let name = osstr_to_lowercase(process.name());
        if targets.contains(&name) {
            println!("Killing {} (pid {})", name, pid);
            // 在新版中 kill() 支持直接传入 Signal
            let _ = process.kill_with(Signal::Kill);
        }
    }
}

/// 将 OsStr 转换为小写字符串
fn osstr_to_lowercase(s: &OsStr) -> String {
    s.to_string_lossy().to_lowercase()
}

/// 递归拷贝（覆盖同名文件）
fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }

    Ok(())
}
