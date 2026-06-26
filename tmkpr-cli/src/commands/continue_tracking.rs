use anyhow::Result;
use tmkpr_lib::config::Config;
use tmkpr_lib::models::entry::{Entry, EntryFilter};
use tmkpr_lib::obsidian_logger;
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::output::{self, ProjectIndex, TaskIndex};

pub fn run(
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    color: bool,
    config: &Config,
) -> Result<()> {
    let svc = EntryService::new(storage, user_id);
    if svc.status()?.is_some() {
        return Err(anyhow::anyhow!(
            "Already tracking an entry. Stop it before continuing."
        ));
    }

    let previous = last_completed_entry(storage, user_id)?;
    let project = previous
        .project_id
        .as_deref()
        .map(|id| storage.get_project(id))
        .transpose()?;
    let task = previous
        .task_id
        .as_deref()
        .map(|id| storage.get_task(id))
        .transpose()?;

    let entry = svc.start(
        project.as_ref().map(|p| p.name.as_str()),
        task.as_ref().map(|t| t.name.as_str()),
        None,
        previous.tags,
        None,
    )?;

    let _ = obsidian_logger::log_activity_to_obsidian(
        config,
        &entry,
        project.as_ref().map(|p| p.name.as_str()),
        task.as_ref().map(|t| t.name.as_str()),
        obsidian_logger::ActivityAction::Started,
    );

    let projects = ProjectIndex(storage.list_projects(user_id, false).unwrap_or_default());
    let tasks = entry
        .project_id
        .as_ref()
        .and_then(|pid| storage.list_tasks(pid, false).ok())
        .unwrap_or_default();

    println!("Continued tracking.");
    output::print_status(&entry, &projects, &TaskIndex(tasks), date_fmt, color);
    Ok(())
}

fn last_completed_entry(storage: &dyn Storage, user_id: &str) -> Result<Entry> {
    EntryService::new(storage, user_id)
        .list(EntryFilter {
            user_id: user_id.to_string(),
            include_active: false,
            limit: Some(1),
            ..Default::default()
        })?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No previous completed entry found to continue."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};
    use tmkpr_lib::models::entry::NewEntry;
    use tmkpr_lib::models::project::NewProject;
    use tmkpr_lib::models::task::NewTask;
    use tmkpr_lib::models::LOCAL_USER_ID;
    use tmkpr_lib::storage::sqlite::SqliteStorage;

    fn mem() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn test_config() -> Config {
        Config::default()
    }

    #[test]
    fn fails_without_previous_completed_entry() {
        let storage = mem();
        let err = run(&storage, LOCAL_USER_ID, "%F %R", false, &test_config()).unwrap_err();

        assert!(err
            .to_string()
            .contains("No previous completed entry found to continue."));
    }

    #[test]
    fn fails_when_entry_is_already_active() {
        let storage = mem();
        let started_at = Utc.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).unwrap();
        storage
            .create_entry(NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: None,
                task_id: None,
                note: None,
                started_at,
                finished_at: None,
                tags: vec![],
            })
            .unwrap();

        let err = run(&storage, LOCAL_USER_ID, "%F %R", false, &test_config()).unwrap_err();

        assert!(err
            .to_string()
            .contains("Already tracking an entry. Stop it before continuing."));
    }

    #[test]
    fn starts_now_with_previous_project_task_and_tags() {
        let storage = mem();
        let project = storage
            .create_project(NewProject {
                user_id: LOCAL_USER_ID.to_string(),
                name: "timekeeper".to_string(),
                description: None,
                color: None,
            })
            .unwrap();
        let task = storage
            .create_task(NewTask {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: project.id.clone(),
                name: "continue command".to_string(),
                description: None,
            })
            .unwrap();
        let started_at = Utc.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).unwrap();
        let finished_at = started_at + Duration::hours(1);
        storage
            .create_entry(NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: Some(project.id.clone()),
                task_id: Some(task.id.clone()),
                note: Some("old note".to_string()),
                started_at,
                finished_at: Some(finished_at),
                tags: vec!["dev".to_string(), "cli".to_string()],
            })
            .unwrap();

        run(&storage, LOCAL_USER_ID, "%F %R", false, &test_config()).unwrap();

        let active = storage.get_active_entry(LOCAL_USER_ID).unwrap().unwrap();
        assert_eq!(active.project_id.as_deref(), Some(project.id.as_str()));
        assert_eq!(active.task_id.as_deref(), Some(task.id.as_str()));
        assert_eq!(active.note, None);
        assert_eq!(active.tags, vec!["dev", "cli"]);
        assert!(active.started_at > finished_at);
        assert_eq!(active.finished_at, None);
    }
}
