# electron-quit-and-install

## 编译

```shell
cargo build --release
```

## 使用

### API说明
|参数|格式|说明|
| --- | --- | --- |
|`--ps`|`--ps={yourApp.exe,yourAppWorker.exe}`|一个以 `,` 为分隔符的应用列表，用于在拷贝文件前退出应用所有相关进程，避免文件占用|
|`--input`|`--input={updateFilePath}`|应用的更新文件存储路径|
|`--output`|`--output={updateDestFilePath}`|应用的更新文件拷贝的目标路径|
|`--app`|`--app={exeName}`|应用的 `exe` 文件路径，用于拷贝文件结束后启动应用|
|`--log`|`--app={logPath}`|更新器日志输出文件路径，如不设置此参数，日志输出至与更新器同级目录下|
|`--ignore`|`--ignore={file1Path,file2Path}`|以 `,` 为分隔符的相对 `--input` 参数路径的文件路径列表，作为拷贝忽略文件列表|

### 示例
```shell
electron-quit-and-install.exe --app="D:\yourApp.exe" --ps="yourApp.exe,otherApp.exe" --input="D:\yourApp\updates" --output="D:\yourApp\resources" --log="D:\yourApp\logs\updater.log"
```

```Javascript
// 在Electron应用中使用
const update_dir = path.join(app.getPath('userData'), 'updates')
const resources_dir = path.join(app.getPath('assets'), 'resources')
const log_path = path.join(app.getPath('logs'), 'updater.log')

const child = spawn(
`${resources_dir}/electron-quit-and-install.exe`,
[
    `--app=${path.resolve(app.getPath('exe'))}`,
    `--ps=yourApp.exe`,
    `--input=${path.resolve(update_dir)}`,
    `--output=${resources_dir}`,
    `--log=${log_path}`
],
{
    detached: true,
    stdio: 'ignore',
    windowsHide: true
}
)
// 允许父进程独立于子进程退出
child.unref()
```