use anyhow::Result;
use tmkpr_lib::models::entry::{Entry, EntryFilter};
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::StatusArgs;
use crate::output::{self, ProjectIndex, TaskIndex};

pub fn run(
    args: StatusArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    format: &str,
    color: bool,
) -> Result<()> {
    let svc = EntryService::new(storage, user_id);
    if args.today {
        let total = svc.today_total()?;
        println!("{}", output::format_duration_minutes(total.num_seconds()));
        return Ok(());
    }

    match svc.status()? {
        None => {
            if args.bar {
                println!("{}", idle_bar_text(storage, user_id)?);
            } else if format == "json" {
                println!("null");
            } else {
                println!("No active tracking session.");
            }
        }
        Some((entry, _elapsed)) => {
            let projects = ProjectIndex(storage.list_projects(user_id, false).unwrap_or_default());
            let tasks = entry
                .project_id
                .as_ref()
                .and_then(|pid| storage.list_tasks(pid, false).ok())
                .unwrap_or_default();
            if args.bar {
                output::print_status_bar(&entry, &projects, &TaskIndex(tasks));
            } else if format == "json" {
                println!("{}", serde_json::to_string_pretty(&entry).unwrap());
            } else {
                output::print_status(&entry, &projects, &TaskIndex(tasks), date_fmt, color);
            }
        }
    }
    Ok(())
}

fn idle_bar_text(storage: &dyn Storage, user_id: &str) -> Result<String> {
    match last_completed_entry(storage, user_id)? {
        Some(entry) => {
            let projects = ProjectIndex(storage.list_projects(user_id, false).unwrap_or_default());
            let tasks = entry
                .project_id
                .as_ref()
                .and_then(|pid| storage.list_tasks(pid, false).ok())
                .unwrap_or_default();
            Ok(format!(
                "▶ {}",
                output::status_bar_label(&entry, &projects, &TaskIndex(tasks))
            ))
        }
        None => Ok("○ Sin tarea activa".to_string()),
    }
}

fn last_completed_entry(storage: &dyn Storage, user_id: &str) -> Result<Option<Entry>> {
    Ok(EntryService::new(storage, user_id)
        .list(EntryFilter {
            user_id: user_id.to_string(),
            include_active: false,
            limit: Some(1),
            ..Default::default()
        })?
        .into_iter()
        .next())
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

    #[test]
    fn idle_bar_shows_spanish_text_without_previous_entry() {
        let storage = mem();

        assert_eq!(
            idle_bar_text(&storage, LOCAL_USER_ID).unwrap(),
            "○ Sin tarea activa"
        );
    }

    #[test]
    fn idle_bar_shows_next_continuable_task() {
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
                name: "1234567890123456789012345".to_string(),
                description: None,
            })
            .unwrap();
        let started_at = Utc.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).unwrap();
        storage
            .create_entry(NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: Some(project.id),
                task_id: Some(task.id),
                note: None,
                started_at,
                finished_at: Some(started_at + Duration::hours(1)),
                tags: vec![],
            })
            .unwrap();

        assert_eq!(
            idle_bar_text(&storage, LOCAL_USER_ID).unwrap(),
            "▶ 12345678901234567890"
        );
    }
}
