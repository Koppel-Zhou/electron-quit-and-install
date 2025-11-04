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

### 示例
```shell
electron-quit-and-install.exe --app="D:\yourApp.exe" --ps="electron-hotupdate-demo.exe" --input="D:\yourApp\updates" --output="D:\yourApp\resources"
```

```Javascript
// 在Electron应用中使用
const update_dir = path.join(app.getPath('userData'), 'updates')
const resources_dir = path.join(app.getPath('assets'), 'resources')

const child = spawn(
`${resources_dir}/electron-quit-and-install.exe`,
[
    `--app=${path.resolve(app.getPath('exe'))}`,
    `--ps=yourApp.exe`,
    `--input=${path.resolve(update_dir)}`,
    `--output=${resources_dir}`
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