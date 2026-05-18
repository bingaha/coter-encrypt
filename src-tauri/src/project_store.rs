pub use coter_core::project_store::Project;

pub fn list_projects() -> Result<Vec<Project>, String> {
 coter_core::project_store::list_projects()
}

pub fn get_project_by_id(id: u64) -> Result<Option<Project>, String> {
 coter_core::project_store::get_project_by_id(id)
}

pub fn get_project_by_name(name: &str) -> Result<Option<Project>, String> {
 coter_core::project_store::get_project_by_name(name)
}

pub fn save_project(project: Project) -> Result<Project, String> {
 coter_core::project_store::save_project(project)
}

pub fn update_project(project: Project) -> Result<Option<Project>, String> {
 coter_core::project_store::update_project(project)
}

pub fn delete_project(id: u64) -> Result<bool, String> {
 coter_core::project_store::delete_project(id)
}

pub fn rename_project(id: u64, new_name: String) -> Result<Option<Project>, String> {
 coter_core::project_store::rename_project(id, new_name)
}
