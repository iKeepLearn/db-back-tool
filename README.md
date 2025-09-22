# 使用说明

## 必要工具

请确保服务器已安装 `7z`。  
安装命令（Debian/Ubuntu）：

```bash
sudo apt install p7zip-full
```

---

## 使用方法

1. 从 [release 页面](https://github.com/iKeepLearn/db-back-tool/releases) 下载可执行文件的 zip 包。
2. 解压后，修改其中的 `config.yaml` 配置文件为正确的配置。

---

## 定时任务（Cron）设置

### 备份任务

每天凌晨 2 点执行数据库备份：

```bash
0 2 * * * /path/to/backupdbtool --config /path/to/config.yaml backup database_name
```
- 替换 `/path/to/backupdbtool` 为实际可执行文件路径
- 替换 `/path/to/config.yaml` 为实际配置文件路径
- 替换 `database_name` 为需要备份的数据库名称


### 上传任务

每天凌晨 2 点 30 分上传所有待上传的备份文件：

```bash
30 2 * * * /path/to/backupdbtool --config /path/to/config.yaml upload --all
```
- 替换路径为实际路径


### 删除任务

每周日凌晨 3 点删除所有已上传的备份文件：

```bash
0 3 * * 0 /path/to/backupdbtool --config /path/to/config.yaml delete --all
```
- 替换路径为实际路径

---

## 手动执行命令示例

- **备份指定数据库：**
  ```bash
  ./backupdbtool --config config.yaml backup database_name
  ```

- **上传所有待上传的备份文件：**
  ```bash
  ./backupdbtool --config config.yaml upload --all
  ```

- **删除所有已上传的备份文件：**
  ```bash
  ./backupdbtool --config config.yaml delete --all
  ```

- **列出所有备份文件：**
  ```bash
  ./backupdbtool --config config.yaml list
  ```

---

如有疑问，请联系开发者。

![联系作者](images/ccwechat.jpg)