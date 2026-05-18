use std::{
 fs,
 path::{Path, PathBuf},
 time::SystemTime,
};

use chrono::{DateTime, Local, NaiveDateTime};
use crc32fast::Hasher;
use directories::ProjectDirs;
use filetime::FileTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
 #[serde(default)]
 pub id: u64,
 pub name: String,
 #[serde(default)]
 pub description: String,
 #[serde(default)]
 pub config: String,
 #[serde(default = "default_status")]
 pub status: i32,
 #[serde(default)]
 pub update_time: String,
}

#[derive(Debug)]
struct ProjectFile {
 path: PathBuf,
 modified: SystemTime,
}

pub fn list_projects() -> Result<Vec<Project>, String> {
 let dir = projects_dir()?;
 list_projects_in_dir(&dir)
}

fn list_projects_in_dir(dir: &Path) -> Result<Vec<Project>, String> {
 let files = list_project_files_sorted(dir)?;
 files
 .iter()
 .map(|file| to_project(&file.path))
 .collect::<Result<Vec<_>, _>>()
}

pub fn get_project_by_id(id: u64) -> Result<Option<Project>, String> {
 let dir = projects_dir()?;
 get_project_by_id_in_dir(&dir, id)
}

fn get_project_by_id_in_dir(dir: &Path, id: u64) -> Result<Option<Project>, String> {
 find_path_by_id(dir, id)?.map_or(Ok(None), |(path, _name)| to_project(&path).map(Some))
}

pub fn get_project_by_name(name: &str) -> Result<Option<Project>, String> {
 let dir = projects_dir()?;
 get_project_by_name_in_dir(&dir, name)
}

fn get_project_by_name_in_dir(dir: &Path, name: &str) -> Result<Option<Project>, String> {
 let path = resolve_by_name_in_dir(dir, name)?;
 if path.is_file() {
 return to_project(&path).map(Some);
 }

 Ok(None)
}

pub fn save_project(project: Project) -> Result<Project, String> {
 let dir = projects_dir()?;
 save_project_in_dir(&dir, project)
}

fn save_project_in_dir(dir: &Path, project: Project) -> Result<Project, String> {
 validate_project_name(&project.name)?;

 if project.config.is_empty() {
 return Err("项目配置不能为空".to_string());
 }

 let path = resolve_by_name_in_dir(dir, &project.name)?;

 if path.exists() {
 return to_project(&path);
 }

 write_new_file(&path, &project.config)?;
 to_project(&path)
}

pub fn update_project(project: Project) -> Result<Option<Project>, String> {
 let dir = projects_dir()?;
 update_project_in_dir(&dir, project)
}

fn update_project_in_dir(dir: &Path, project: Project) -> Result<Option<Project>, String> {
 let origin = find_path_by_id(dir, project.id)?;

 match origin {
 Some((origin_path, origin_name)) => {
 let target_name = if project.name.trim().is_empty() {
 origin_name
 } else {
 project.name
 };
 validate_project_name(&target_name)?;

 let target_path = resolve_by_name_in_dir(dir, &target_name)?;
 if origin_path != target_path {
 if target_path.exists() {
 return Err(format!("目标项目名已存在: {target_name}"));
 }

 fs::rename(&origin_path, &target_path).map_err(|error| {
 format!(
 "重命名项目失败 {} -> {}: {error}",
 origin_path.display(),
 target_path.display()
 )
 })?;
 touch_file(&target_path)?;
 }

 if !project.config.is_empty() {
 write_existing_file(&target_path, &project.config)?;
 }

 to_project(&target_path).map(Some)
 }
 None => {
 if project.name.trim().is_empty() {
 return Ok(None);
 }

 validate_project_name(&project.name)?;
 let target_path = resolve_by_name_in_dir(dir, &project.name)?;

 if target_path.exists() {
 write_existing_file(&target_path, &project.config)?;
 } else {
 write_new_file(&target_path, &project.config)?;
 }

 touch_file(&target_path)?;
 to_project(&target_path).map(Some)
 }
 }
}

pub fn delete_project(id: u64) -> Result<bool, String> {
 let dir = projects_dir()?;
 delete_project_in_dir(&dir, id)
}

fn delete_project_in_dir(dir: &Path, id: u64) -> Result<bool, String> {
 if let Some((path, _name)) = find_path_by_id(dir, id)? {
 fs::remove_file(&path)
 .map_err(|error| format!("删除项目失败 {}: {error}", path.display()))?;
 return Ok(true);
 }

 Ok(false)
}

pub fn rename_project(id: u64, new_name: String) -> Result<Option<Project>, String> {
 let dir = projects_dir()?;
 rename_project_in_dir(&dir, id, new_name)
}

fn rename_project_in_dir(dir: &Path, id: u64, new_name: String) -> Result<Option<Project>, String> {
 let Some((origin_path, _origin_name)) = find_path_by_id(dir, id)? else {
 return Ok(None);
 };

 validate_project_name(&new_name)?;
 let target_path = resolve_by_name_in_dir(dir, &new_name)?;

 if origin_path == target_path {
 return to_project(&target_path).map(Some);
 }

 if target_path.exists() {
 return Err(format!("目标项目名已存在: {new_name}"));
 }

 fs::rename(&origin_path, &target_path).map_err(|error| {
 format!(
 "重命名项目失败 {} -> {}: {error}",
 origin_path.display(),
 target_path.display()
 )
 })?;
 touch_file(&target_path)?;

 to_project(&target_path).map(Some)
}

fn projects_dir() -> Result<PathBuf, String> {
 projects_dir_from_app_config()
}

fn projects_dir_from_app_config() -> Result<PathBuf, String> {
 let dirs = ProjectDirs::from("com", "coter", "CoterEncrypt")
 .ok_or_else(|| "无法定位应用配置目录".to_string())?;
 let dir = dirs.config_dir().join("projects");

 ensure_projects_dir(dir)
}

fn ensure_projects_dir(dir: PathBuf) -> Result<PathBuf, String> {
 fs::create_dir_all(&dir)
 .map_err(|error| format!("无法创建项目目录 {}: {error}", dir.display()))?;

 Ok(dir)
}

fn list_project_files_sorted(dir: &Path) -> Result<Vec<ProjectFile>, String> {
 let entries = fs::read_dir(dir)
 .map_err(|error| format!("读取项目目录失败 {}: {error}", dir.display()))?;
 let mut files = Vec::new();

 for entry in entries {
 let entry = entry.map_err(|error| format!("读取项目目录项失败: {error}"))?;
 let path = entry.path();

 if !path.is_file() {
 continue;
 }

 if path
 .file_name()
 .and_then(|file_name| file_name.to_str())
 .is_some_and(|file_name| file_name.starts_with('.'))
 {
 continue;
 }

 let modified = entry
 .metadata()
 .and_then(|metadata| metadata.modified())
 .unwrap_or(SystemTime::UNIX_EPOCH);

 files.push(ProjectFile { path, modified });
 }

 files.sort_by(|a, b| b.modified.cmp(&a.modified));
 Ok(files)
}

fn resolve_by_name_in_dir(dir: &Path, name: &str) -> Result<PathBuf, String> {
 let exact = dir.join(name);

 if exact.is_file() {
 return Ok(exact);
 }

 let json = dir.join(format!("{name}.json"));
 if json.is_file() {
 return Ok(json);
 }

 Ok(json)
}

fn find_path_by_id(dir: &Path, id: u64) -> Result<Option<(PathBuf, String)>, String> {
 for file in list_project_files_sorted(dir)? {
 let name = display_name(&file.path)?;
 if id_from_name(&name) == id {
 return Ok(Some((file.path, name)));
 }
 }

 Ok(None)
}

fn to_project(path: &Path) -> Result<Project, String> {
 let name = display_name(path)?;
 let metadata = fs::metadata(path)
 .map_err(|error| format!("读取项目文件元数据失败 {}: {error}", path.display()))?;
 let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
 let content = fs::read_to_string(path)
 .map_err(|error| format!("读取项目文件失败 {}: {error}", path.display()))?;

 Ok(Project {
 id: id_from_name(&name),
 name,
 description: String::new(),
 config: content,
 status: 1,
 update_time: format_update_time(modified),
 })
}

fn display_name(path: &Path) -> Result<String, String> {
 let file_name = path
 .file_name()
 .and_then(|file_name| file_name.to_str())
 .ok_or_else(|| format!("项目文件名不是有效 UTF-8: {}", path.display()))?;

 Ok(file_name
 .strip_suffix(".json")
 .unwrap_or(file_name)
 .to_string())
}

fn id_from_name(name: &str) -> u64 {
 let mut hasher = Hasher::new();
 hasher.update(name.as_bytes());
 u64::from(hasher.finalize())
}

fn default_status() -> i32 {
 1
}

fn validate_project_name(name: &str) -> Result<(), String> {
 if name.trim().is_empty() {
 return Err("项目名称不能为空".to_string());
 }

 if name.contains('/') || name.contains('\\') {
 return Err("项目名称不能包含路径分隔符".to_string());
 }

 if Path::new(name)
 .file_name()
 .and_then(|file_name| file_name.to_str())
 != Some(name)
 {
 return Err("项目名称不能包含路径".to_string());
 }

 Ok(())
}

fn write_new_file(path: &Path, config: &str) -> Result<(), String> {
 fs::write(path, config).map_err(|error| format!("保存项目失败 {}: {error}", path.display()))
}

fn write_existing_file(path: &Path, config: &str) -> Result<(), String> {
 fs::write(path, config).map_err(|error| format!("更新项目失败 {}: {error}", path.display()))
}

fn touch_file(path: &Path) -> Result<(), String> {
 let now = FileTime::now();
 filetime::set_file_mtime(path, now)
 .map_err(|error| format!("刷新项目修改时间失败 {}: {error}", path.display()))
}

fn format_update_time(system_time: SystemTime) -> String {
 let local_time: DateTime<Local> = system_time.into();
 let naive: NaiveDateTime = local_time.naive_local();

 naive.format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
 use std::fs;

 use super::{
 delete_project_in_dir, get_project_by_id_in_dir, get_project_by_name_in_dir, id_from_name,
 list_projects_in_dir, rename_project_in_dir, save_project_in_dir, update_project_in_dir,
 Project,
 };

 fn project(name: &str, config: &str) -> Project {
 Project {
 id: id_from_name(name),
 name: name.to_string(),
 description: String::new(),
 config: config.to_string(),
 status: 1,
 update_time: String::new(),
 }
 }

 #[test]
 fn crc32_project_id_matches_expected_unsigned_int() {
 assert_eq!(id_from_name("安徽SSO"), 730_391_273);
 assert_eq!(id_from_name("临沂加密"), 3_803_985_762);
 }

 #[test]
 fn save_returns_existing_project_without_overwriting() {
 let dir = tempfile::tempdir().unwrap();

 let saved = save_project_in_dir(dir.path(), project("项目A", "{\"value\":1}")).unwrap();
 let existing = save_project_in_dir(dir.path(), project("项目A", "{\"value\":2}")).unwrap();

 assert_eq!(saved.id, existing.id);
 assert_eq!(existing.name, "项目A");
 assert_eq!(existing.config, "{\"value\":1}");
 assert_eq!(
 fs::read_to_string(dir.path().join("项目A.json")).unwrap(),
 "{\"value\":1}"
 );
 }

 #[test]
 fn list_get_update_rename_and_delete_project_files() {
 let dir = tempfile::tempdir().unwrap();
 let saved = save_project_in_dir(dir.path(), project("项目A", "{\"value\":1}")).unwrap();

 let by_id = get_project_by_id_in_dir(dir.path(), saved.id)
 .unwrap()
 .unwrap();
 assert_eq!(by_id.name, "项目A");

 let by_name = get_project_by_name_in_dir(dir.path(), "项目A")
 .unwrap()
 .unwrap();
 assert_eq!(by_name.config, "{\"value\":1}");

 let updated = update_project_in_dir(
 dir.path(),
 Project {
 config: "{\"value\":2}".to_string(),
 ..project("项目A", "")
 },
 )
 .unwrap()
 .unwrap();
 assert_eq!(updated.config, "{\"value\":2}");

 let renamed = rename_project_in_dir(dir.path(), saved.id, "项目B".to_string())
 .unwrap()
 .unwrap();
 assert_eq!(renamed.name, "项目B");
 assert!(!dir.path().join("项目A.json").exists());
 assert!(dir.path().join("项目B.json").exists());

 let listed = list_projects_in_dir(dir.path()).unwrap();
 assert_eq!(listed.len(), 1);
 assert_eq!(listed[0].name, "项目B");

 assert!(delete_project_in_dir(dir.path(), renamed.id).unwrap());
 assert!(list_projects_in_dir(dir.path()).unwrap().is_empty());
 }

 #[test]
 fn rename_rejects_existing_target_name() {
 let dir = tempfile::tempdir().unwrap();
 let source = save_project_in_dir(dir.path(), project("项目A", "{\"value\":1}")).unwrap();
 save_project_in_dir(dir.path(), project("项目B", "{\"value\":2}")).unwrap();

 let error = rename_project_in_dir(dir.path(), source.id, "项目B".to_string()).unwrap_err();

 assert!(error.contains("目标项目名已存在"));
 assert_eq!(
 fs::read_to_string(dir.path().join("项目A.json")).unwrap(),
 "{\"value\":1}"
 );
 assert_eq!(
 fs::read_to_string(dir.path().join("项目B.json")).unwrap(),
 "{\"value\":2}"
 );
 }
}
