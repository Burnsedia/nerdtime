// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use nerdtime_db as db;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert(InsertTarget),
    Command(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertTarget {
    Filter,
    NewSessionProject,
    NewSessionDescription,
    NewTaskTitle,
    NewTaskEstimate,
    NewTaskLabels,
    DevlogSearch,
    AdvisorTime,
    AdvisorEnergy,
    AdvisorBlocked,
    NewDevlogTitle,
    NewDevlogRole,
    NewDevlogTags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Dashboard,
    Stats,
    Tasks,
    Matrix,
    Devlog,
    Advisor,
}

#[derive(Debug, Clone)]
pub enum Modal {
    NewSession,
    NewTask,
    Help,
    Confirm { message: String, action: ConfirmAction },
    FilterInput,
    AdvisorForm,
    TaskDetail(usize),
    DevlogDetail(usize),
    Heatmap,
    Insights,
    NewDevlogEntry,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    Quit,
    DeleteSession(usize),
    DeleteTask(usize),
}

#[derive(Debug, Clone)]
pub enum SyncStatus {
    Idle,
    Syncing,
    Success(usize),
    Failure(String),
    NoConfig,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub style: ToastStyle,
    pub expires_at: std::time::Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastStyle {
    Info,
    Success,
    Error,
}

pub struct App {
    pub mode: Mode,
    pub active_panel: Panel,
    pub active_session: Option<db::Session>,
    pub sessions: Vec<db::Session>,
    pub tasks: Vec<db::TaskRow>,
    pub stats: Vec<db::ProjectStat>,
    pub devlog_entries: Vec<db::DevlogEntry>,
    pub advisor_result: Option<db::Advice>,
    pub active_modal: Option<Modal>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub filter_text: String,
    pub toast: Option<Toast>,
    pub terminal_size: (u16, u16),
    pub session_start_instant: Option<std::time::Instant>,
    pub elapsed_seconds: u64,
    pub last_db_refresh: std::time::Instant,
    pub sync_status: SyncStatus,
    pub last_sync: Option<String>,
    pub api_url: String,
    pub has_token: bool,
    pub unsynced_count: usize,
    pub insert_buffer: String,
    pub command_buffer: String,
    pub new_session_project: String,
    pub new_session_desc: String,
    pub new_task_title: String,
    pub new_task_estimate: String,
    pub new_task_labels: String,
    pub advisor_time: String,
    pub advisor_energy: String,
    pub advisor_blocked: String,
    pub devlog_search_query: String,
    pub filter_text_prev: String,
    pub new_devlog_title: String,
    pub new_devlog_role: String,
    pub new_devlog_tags: String,
    pub last_tick: std::time::Instant,
    pub heatmap_data: Vec<db::HeatmapCell>,
    pub insights_data: Option<db::Insights>,
    pub total_sessions_count: usize,
    pub total_duration: i64,
    pub project_count: usize,
    pub days: i64,
}

impl App {
    pub fn new(conn: &db::Connection, config: &Option<crate::config::Config>) -> Self {
        let active_session = db::show_status(conn).ok().flatten();
        let sessions = db::list_sessions(conn, None, 50).unwrap_or_default();
        let tasks = db::list_tasks(conn, None, Some("active")).unwrap_or_default();
        let stats = db::stats_by_project(conn).unwrap_or_default();
        let devlog_entries = db::list_devlog_entries(conn, 20).unwrap_or_default();
        let session_start_instant = active_session.as_ref().map(|_| std::time::Instant::now());
        let elapsed_seconds = active_session
            .as_ref()
            .map(|s| {
                let now = chrono::Utc::now();
                let dur = now - s.started_at;
                dur.num_seconds().max(0) as u64
            })
            .unwrap_or(0);

        let (api_url, has_token) = match config {
            Some(ref c) => (c.api_url.clone(), c.token.is_some()),
            None => ("http://localhost:3000/api".to_string(), false),
        };

        let unsynced = db::get_unsynced_sessions(conn).unwrap_or_default();
        let total_sessions_count = sessions.len();
        let project_count = stats.len();
        let total_duration: i64 = stats.iter().map(|s| s.total_seconds).sum();

        Self {
            mode: Mode::Normal,
            active_panel: Panel::Dashboard,
            active_session,
            sessions,
            tasks,
            stats,
            devlog_entries,
            advisor_result: None,
            active_modal: None,
            selected_index: 0,
            scroll_offset: 0,
            filter_text: String::new(),
            toast: None,
            terminal_size: (0, 0),
            session_start_instant,
            elapsed_seconds,
            last_db_refresh: std::time::Instant::now(),
            sync_status: SyncStatus::Idle,
            last_sync: None,
            api_url,
            has_token,
            unsynced_count: unsynced.len(),
            insert_buffer: String::new(),
            command_buffer: String::new(),
            new_session_project: String::new(),
            new_session_desc: String::new(),
            new_task_title: String::new(),
            new_task_estimate: String::new(),
            new_task_labels: String::new(),
            advisor_time: String::new(),
            advisor_energy: String::new(),
            advisor_blocked: String::new(),
            devlog_search_query: String::new(),
            filter_text_prev: String::new(),
            new_devlog_title: String::new(),
            new_devlog_role: String::new(),
            new_devlog_tags: String::new(),
            last_tick: std::time::Instant::now(),
            heatmap_data: Vec::new(),
            insights_data: None,
            total_sessions_count,
            total_duration,
            project_count,
            days: 30,
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent, conn: &db::Connection) -> Result<bool> {
        

        // Insert mode must be handled even when a modal is active (form fields)
        if let Mode::Insert(ref target) = self.mode {
            return self.handle_insert_key(key, target.clone(), conn);
        }

        if self.active_modal.is_some() {
            return self.handle_modal_key(key, conn);
        }

        match self.mode {
            Mode::Normal => self.handle_normal_key(key, conn),
            Mode::Command(_) => self.handle_command_key(key, conn),
            Mode::Insert(_) => unreachable!(),
        }
    }

    fn handle_modal_key(&mut self, key: crossterm::event::KeyEvent, conn: &db::Connection) -> Result<bool> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                self.active_modal = None;
            }
            KeyCode::Enter => {
                let modal = self.active_modal.clone();
                match modal {
                    Some(Modal::Confirm { message: _, ref action }) => {
                        match action {
                            ConfirmAction::Quit => {
                                return Ok(true);
                            }
                            ConfirmAction::DeleteSession(_idx) => {
                                self.active_modal = None;
                                self.refresh_all(conn);
                            }
                            ConfirmAction::DeleteTask(_idx) => {
                                self.active_modal = None;
                                self.refresh_all(conn);
                            }
                        }
                    }
                    Some(Modal::NewSession) => {
                        let project = self.new_session_project.clone();
                        if !project.is_empty() {
                            let _ = db::start_session(
                                conn,
                                &project,
                                if self.new_session_desc.is_empty() {
                                    None
                                } else {
                                    Some(&self.new_session_desc)
                                },
                                None,
                                None,
                                None,
                            );
                            self.new_session_project.clear();
                            self.new_session_desc.clear();
                            self.active_modal = None;
                            self.refresh_all(conn);
                        }
                    }
                    Some(Modal::NewTask) => {
                        let title = self.new_task_title.clone();
                        if !title.is_empty() {
                            let est = if self.new_task_estimate.is_empty() {
                                None
                            } else {
                                db::parse_duration(&self.new_task_estimate).ok().flatten()
                            };
                            let _ = db::add_task(
                                conn,
                                "default",
                                &title,
                                None,
                                est,
                                3,
                                3,
                                if self.new_task_labels.is_empty() {
                                    None
                                } else {
                                    Some(&self.new_task_labels)
                                },
                                None,
                                None,
                            );
                            self.new_task_title.clear();
                            self.new_task_estimate.clear();
                            self.new_task_labels.clear();
                            self.active_modal = None;
                            self.refresh_all(conn);
                        }
                    }
                    Some(Modal::AdvisorForm) => {
                        let time = self.advisor_time.clone();
                        let energy = self.advisor_energy.clone();
                        if !time.is_empty() && !energy.is_empty() {
                            let available_seconds =
                                db::parse_duration(&time).ok().flatten().unwrap_or(3600);
                            let input = db::AdvisorInput {
                                available_seconds,
                                energy: energy.clone(),
                                blocked: if self.advisor_blocked.is_empty() {
                                    None
                                } else {
                                    Some(self.advisor_blocked.clone())
                                },
                            };
                            self.advisor_result = db::decide(conn, &input).ok();
                            self.active_modal = None;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_normal_key(&mut self, key: crossterm::event::KeyEvent, conn: &db::Connection) -> Result<bool> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') => {
                if self.active_session.is_some() {
                    self.active_modal = Some(Modal::Confirm {
                        message: "Active session running. Quit anyway?".to_string(),
                        action: ConfirmAction::Quit,
                    });
                } else {
                    return Ok(true);
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected_index = self.selected_index.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            KeyCode::Char('h') => {
                match self.active_panel {
                    Panel::Matrix => {
                        self.active_panel = Panel::Tasks;
                        self.refresh_panel(conn);
                    }
                    Panel::Stats => {
                        let data = db::heatmap_data(conn, self.days, None).ok().unwrap_or_default();
                        self.heatmap_data = data;
                        self.active_modal = Some(Modal::Heatmap);
                    }
                    _ => {}
                }
            }
            KeyCode::Char('l') => {
                if self.active_panel == Panel::Tasks {
                    self.active_panel = Panel::Matrix;
                    self.refresh_panel(conn);
                }
            }
            KeyCode::Char('n') => {
                match self.active_panel {
                    Panel::Dashboard | Panel::Stats => {
                        self.active_modal = Some(Modal::NewSession);
                        self.mode = Mode::Insert(InsertTarget::NewSessionProject);
                    }
                    Panel::Tasks => {
                        self.active_modal = Some(Modal::NewTask);
                        self.mode = Mode::Insert(InsertTarget::NewTaskTitle);
                    }
                    Panel::Devlog => {
                        self.active_modal = Some(Modal::NewDevlogEntry);
                        self.mode = Mode::Insert(InsertTarget::NewDevlogTitle);
                    }
                    _ => {}
                }
            }
            KeyCode::Char('s') => {
                if self.active_panel == Panel::Advisor {
                    if let Some(ref advice) = self.advisor_result {
                        if let Some(ref task_id) = advice.task_id {
                            if let Some(task) = self.tasks.iter().find(|t| &t.id == task_id) {
                                let _ = db::start_session(
                                    conn,
                                    &task.project_name,
                                    None,
                                    Some(&task.id),
                                    task.estimated_seconds,
                                    task.labels.as_deref(),
                                );
                                self.refresh_all(conn);
                            }
                        }
                    }
                } else {
                    self.sync_sessions(conn);
                }
            }
            KeyCode::Char('r') => {
                self.refresh_all(conn);
            }
            KeyCode::Char('?') => {
                self.active_modal = Some(Modal::Help);
            }
            KeyCode::Char(':') => {
                self.mode = Mode::Command(String::new());
                self.command_buffer.clear();
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Insert(InsertTarget::Filter);
                self.insert_buffer.clear();
            }
            KeyCode::Tab | KeyCode::Char('g') if key.modifiers == crossterm::event::KeyModifiers::NONE => {
                self.next_panel(conn);
            }
            KeyCode::BackTab => {
                self.prev_panel(conn);
            }
            KeyCode::Enter => {
                match self.active_panel {
                    Panel::Dashboard => {
                        if self.active_session.is_some() {
                            let _ = db::stop_session(conn);
                            self.refresh_all(conn);
                        }
                    }
                    Panel::Tasks => {
                        if !self.tasks.is_empty() && self.selected_index < self.tasks.len() {
                            if self.active_session.is_some() {
                                let _ = db::stop_session(conn);
                            }
                            let task = &self.tasks[self.selected_index];
                            let _ = db::start_session(
                                conn,
                                &task.project_name,
                                None,
                                Some(&task.id),
                                task.estimated_seconds,
                                task.labels.as_deref(),
                            );
                            self.refresh_all(conn);
                        }
                    }
                    Panel::Matrix => {
                        if !self.tasks.is_empty() && self.selected_index < self.tasks.len() {
                            self.active_modal = Some(Modal::TaskDetail(self.selected_index));
                        }
                    }
                    Panel::Devlog => {
                        if !self.devlog_entries.is_empty() && self.selected_index < self.devlog_entries.len() {
                            self.active_modal = Some(Modal::DevlogDetail(self.selected_index));
                        }
                    }
                    Panel::Stats => {
                        if !self.stats.is_empty() && self.selected_index < self.stats.len() {
                            if self.active_session.is_some() {
                                let _ = db::stop_session(conn);
                            }
                            let _ = db::start_session(
                                conn,
                                &self.stats[self.selected_index].project,
                                None,
                                None,
                                None,
                                None,
                            );
                            self.refresh_all(conn);
                        }
                    }
                    Panel::Advisor => {
                        self.active_modal = Some(Modal::AdvisorForm);
                        self.mode = Mode::Insert(InsertTarget::AdvisorTime);
                    }
                    _ => {}
                }
            }
            KeyCode::Esc => {
                if self.active_modal.is_some() {
                    self.active_modal = None;
                }
            }
            KeyCode::Char('a') if self.active_panel == Panel::Tasks => {
                self.active_modal = Some(Modal::NewTask);
                self.mode = Mode::Insert(InsertTarget::NewTaskTitle);
            }
            KeyCode::Char('c') if self.active_panel == Panel::Tasks => {
                if !self.tasks.is_empty() && self.selected_index < self.tasks.len() {
                    let task = &self.tasks[self.selected_index];
                    let _ = db::complete_task(conn, &task.id);
                    self.refresh_all(conn);
                }
            }
            KeyCode::Char('x') if self.active_panel == Panel::Tasks => {
                if !self.tasks.is_empty() && self.selected_index < self.tasks.len() {
                    let task = &self.tasks[self.selected_index];
                    let _ = db::cancel_task(conn, &task.id);
                    self.refresh_all(conn);
                }
            }
            KeyCode::Char('m') if self.active_panel == Panel::Tasks => {
                self.active_panel = Panel::Matrix;
                self.selected_index = 0;
                self.refresh_panel(conn);
            }
            KeyCode::Char('i') if self.active_panel == Panel::Stats => {
                let data = db::insights_data(conn, self.days, None).ok();
                self.insights_data = data;
                self.active_modal = Some(Modal::Insights);
            }
            KeyCode::Char('e') if self.active_panel == Panel::Devlog => {
                if !self.devlog_entries.is_empty() && self.selected_index < self.devlog_entries.len() {
                    let entry = &self.devlog_entries[self.selected_index];
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(format!("$EDITOR {}", entry.id))
                        .spawn();
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_insert_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        target: InsertTarget,
        conn: &db::Connection,
    ) -> Result<bool> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.active_modal = None;
            }
            KeyCode::Enter => {
                let text = self.insert_buffer.clone();
                match target {
                    InsertTarget::Filter => {
                        self.filter_text = text;
                        self.insert_buffer.clear();
                        self.mode = Mode::Normal;
                    }
                    InsertTarget::NewSessionProject => {
                        self.new_session_project = text;
                        self.insert_buffer.clear();
                        self.mode = Mode::Insert(InsertTarget::NewSessionDescription);
                    }
                    InsertTarget::NewSessionDescription => {
                        self.new_session_desc = text;
                        if !self.new_session_project.is_empty() {
                            let _ = db::start_session(
                                conn,
                                &self.new_session_project,
                                if self.new_session_desc.is_empty() {
                                    None
                                } else {
                                    Some(&self.new_session_desc)
                                },
                                None,
                                None,
                                None,
                            );
                            self.new_session_project.clear();
                            self.new_session_desc.clear();
                            self.insert_buffer.clear();
                            self.active_modal = None;
                            self.mode = Mode::Normal;
                            self.refresh_all(conn);
                        }
                    }
                    InsertTarget::NewTaskTitle => {
                        self.new_task_title = text;
                        self.insert_buffer.clear();
                        self.mode = Mode::Insert(InsertTarget::NewTaskEstimate);
                    }
                    InsertTarget::NewTaskEstimate => {
                        self.new_task_estimate = text;
                        self.insert_buffer.clear();
                        self.mode = Mode::Insert(InsertTarget::NewTaskLabels);
                    }
                    InsertTarget::NewTaskLabels => {
                        self.new_task_labels = text;
                        if !self.new_task_title.is_empty() {
                            let est = if self.new_task_estimate.is_empty() {
                                None
                            } else {
                                db::parse_duration(&self.new_task_estimate).ok().flatten()
                            };
                            let _ = db::add_task(
                                conn,
                                "default",
                                &self.new_task_title,
                                None,
                                est,
                                3,
                                3,
                                if self.new_task_labels.is_empty() {
                                    None
                                } else {
                                    Some(&self.new_task_labels)
                                },
                                None,
                                None,
                            );
                            self.new_task_title.clear();
                            self.new_task_estimate.clear();
                            self.new_task_labels.clear();
                            self.insert_buffer.clear();
                            self.active_modal = None;
                            self.mode = Mode::Normal;
                            self.refresh_all(conn);
                        }
                    }
                    InsertTarget::DevlogSearch => {
                        if text.is_empty() {
                            self.devlog_search_query.clear();
                            self.devlog_entries =
                                db::list_devlog_entries(conn, 20).unwrap_or_default();
                        } else {
                            self.devlog_search_query = text.clone();
                            self.devlog_entries =
                                db::search_devlog_entries(conn, &text, None).unwrap_or_default();
                        }
                        self.selected_index = 0;
                        self.mode = Mode::Normal;
                        self.insert_buffer.clear();
                    }
                    InsertTarget::AdvisorTime => {
                        self.advisor_time = text;
                        self.insert_buffer.clear();
                        self.mode = Mode::Insert(InsertTarget::AdvisorEnergy);
                    }
                    InsertTarget::AdvisorEnergy => {
                        self.advisor_energy = text;
                        self.insert_buffer.clear();
                        self.mode = Mode::Insert(InsertTarget::AdvisorBlocked);
                    }
                    InsertTarget::AdvisorBlocked => {
                        self.advisor_blocked = text;
                        let time = self.advisor_time.clone();
                        let energy = self.advisor_energy.clone();
                        if !time.is_empty() && !energy.is_empty() {
                            let available_seconds =
                                db::parse_duration(&time).ok().flatten().unwrap_or(3600);
                            let input = db::AdvisorInput {
                                available_seconds,
                                energy: energy.clone(),
                                blocked: if self.advisor_blocked.is_empty() {
                                    None
                                } else {
                                    Some(self.advisor_blocked.clone())
                                },
                            };
                            self.advisor_result = db::decide(conn, &input).ok();
                            self.active_modal = None;
                            self.mode = Mode::Normal;
                        }
                    }
                    InsertTarget::NewDevlogTitle => {
                        self.new_devlog_title = text;
                        self.insert_buffer.clear();
                        self.mode = Mode::Insert(InsertTarget::NewDevlogRole);
                    }
                    InsertTarget::NewDevlogRole => {
                        self.new_devlog_role = text;
                        self.insert_buffer.clear();
                        self.mode = Mode::Insert(InsertTarget::NewDevlogTags);
                    }
                    InsertTarget::NewDevlogTags => {
                        self.new_devlog_tags = text;
                        if !self.new_devlog_title.is_empty() {
                            let tags: Vec<String> = self
                                .new_devlog_tags
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            let entry = nerdtime_core::DevlogEntry {
                                id: uuid::Uuid::new_v4().to_string(),
                                date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                                title: self.new_devlog_title.clone(),
                                role: self.new_devlog_role.clone(),
                                tags,
                                context: String::new(),
                                changes: vec![],
                                decisions: vec![],
                                commits: vec![],
                                session_id: None,
                                created_at: chrono::Utc::now().to_rfc3339(),
                            };
                            let _ = db::insert_devlog_entry(conn, &entry);
                            self.refresh_all(conn);
                        }
                        self.new_devlog_title.clear();
                        self.new_devlog_role.clear();
                        self.new_devlog_tags.clear();
                        self.insert_buffer.clear();
                        self.active_modal = None;
                        self.mode = Mode::Normal;
                    }
                }
            }
            KeyCode::Char(c) => {
                self.insert_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.insert_buffer.pop();
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_command_key(&mut self, key: crossterm::event::KeyEvent, conn: &db::Connection) -> Result<bool> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                let cmd = self.command_buffer.trim().to_lowercase();
                self.mode = Mode::Normal;
                match cmd.as_str() {
                    "q" | "quit" => {
                        if self.active_session.is_some() {
                            self.active_modal = Some(Modal::Confirm {
                                message: "Active session running. Quit anyway?".to_string(),
                                action: ConfirmAction::Quit,
                            });
                        } else {
                            return Ok(true);
                        }
                    }
                    "w" | "sync" => {
                        self.sync_sessions(conn);
                    }
                    "wq" => {
                        self.sync_sessions(conn);
                        if self.active_session.is_some() {
                            self.active_modal = Some(Modal::Confirm {
                                message: "Active session running. Quit anyway?".to_string(),
                                action: ConfirmAction::Quit,
                            });
                        } else {
                            return Ok(true);
                        }
                    }
                    "e" | "refresh" => {
                        self.refresh_all(conn);
                    }
                    "help" => {
                        self.active_modal = Some(Modal::Help);
                    }
                    "new" => {
                        self.active_modal = Some(Modal::NewSession);
                        self.mode = Mode::Insert(InsertTarget::NewSessionProject);
                    }
                    "dashboard" => {
                        self.active_panel = Panel::Dashboard;
                        self.selected_index = 0;
                        self.refresh_panel(conn);
                    }
                    "stats" => {
                        self.active_panel = Panel::Stats;
                        self.selected_index = 0;
                        self.refresh_panel(conn);
                    }
                    "tasks" => {
                        self.active_panel = Panel::Tasks;
                        self.selected_index = 0;
                        self.refresh_panel(conn);
                    }
                    "matrix" => {
                        self.active_panel = Panel::Matrix;
                        self.selected_index = 0;
                        self.refresh_panel(conn);
                    }
                    "devlog" => {
                        self.active_panel = Panel::Devlog;
                        self.selected_index = 0;
                        self.refresh_panel(conn);
                    }
                    "advisor" => {
                        self.active_panel = Panel::Advisor;
                        self.selected_index = 0;
                        self.refresh_panel(conn);
                    }
                    d if d.starts_with("days ") => {
                        if let Ok(n) = d[5..].trim().parse::<i64>() {
                            self.days = n;
                            self.refresh_all(conn);
                        }
                    }
                    _ => {
                        self.toast = Some(Toast {
                            message: format!("Unknown command: {}", cmd),
                            style: ToastStyle::Error,
                            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(3),
                        });
                    }
                }
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
            }
            _ => {}
        }
        Ok(false)
    }

    pub fn tick(&mut self) {
        if let Some(ref start) = self.session_start_instant {
            self.elapsed_seconds = start.elapsed().as_secs();
        }

        if let Some(ref toast) = self.toast {
            if std::time::Instant::now() >= toast.expires_at {
                self.toast = None;
            }
        }
    }

    pub fn refresh_if_needed(&mut self, conn: &db::Connection) {
        if self.active_modal.is_some() {
            return;
        }
        let now = std::time::Instant::now();
        if now.duration_since(self.last_db_refresh).as_secs() >= 5 {
            self.refresh_all(conn);
        }
    }

    pub fn refresh_all(&mut self, conn: &db::Connection) {
        self.active_session = db::show_status(conn).ok().flatten();
        self.sessions = db::list_sessions(conn, None, 50).unwrap_or_default();
        self.tasks = db::list_tasks(conn, None, Some("active")).unwrap_or_default();
        self.stats = db::stats_by_project(conn).unwrap_or_default();
        self.devlog_entries = db::list_devlog_entries(conn, 20).unwrap_or_default();
        let unsynced = db::get_unsynced_sessions(conn).unwrap_or_default();
        self.unsynced_count = unsynced.len();
        self.total_sessions_count = self.sessions.len();
        self.total_duration = self.stats.iter().map(|s| s.total_seconds).sum();
        self.project_count = self.stats.len();
        self.heatmap_data = db::heatmap_data(conn, self.days, None).ok().unwrap_or_default();
        self.insights_data = db::insights_data(conn, self.days, None).ok();
        self.last_db_refresh = std::time::Instant::now();

        if self.active_session.is_some() && self.session_start_instant.is_none() {
            self.session_start_instant = Some(std::time::Instant::now());
        } else if self.active_session.is_none() {
            self.session_start_instant = None;
            self.elapsed_seconds = 0;
        }
    }

    pub fn refresh_panel(&mut self, conn: &db::Connection) {
        match self.active_panel {
            Panel::Dashboard => {
                self.active_session = db::show_status(conn).ok().flatten();
                self.sessions = db::list_sessions(conn, None, 50).unwrap_or_default();
                self.heatmap_data = db::heatmap_data(conn, self.days, None).ok().unwrap_or_default();
                self.insights_data = db::insights_data(conn, self.days, None).ok();
            }
            Panel::Stats => {
                self.stats = db::stats_by_project(conn).unwrap_or_default();
            }
            Panel::Tasks | Panel::Matrix => {
                self.tasks = db::list_tasks(conn, None, Some("active")).unwrap_or_default();
            }
            Panel::Devlog => {
                self.devlog_entries = db::list_devlog_entries(conn, 20).unwrap_or_default();
            }
            Panel::Advisor => {}
        }
    }

    pub fn next_panel(&mut self, conn: &db::Connection) {
        self.active_panel = match self.active_panel {
            Panel::Dashboard => Panel::Stats,
            Panel::Stats => Panel::Tasks,
            Panel::Tasks => Panel::Matrix,
            Panel::Matrix => Panel::Devlog,
            Panel::Devlog => Panel::Advisor,
            Panel::Advisor => Panel::Dashboard,
        };
        self.selected_index = 0;
        self.refresh_panel(conn);
    }

    pub fn prev_panel(&mut self, conn: &db::Connection) {
        self.active_panel = match self.active_panel {
            Panel::Dashboard => Panel::Advisor,
            Panel::Stats => Panel::Dashboard,
            Panel::Tasks => Panel::Stats,
            Panel::Matrix => Panel::Tasks,
            Panel::Devlog => Panel::Matrix,
            Panel::Advisor => Panel::Devlog,
        };
        self.selected_index = 0;
        self.refresh_panel(conn);
    }

    fn sync_sessions(&mut self, conn: &db::Connection) {
        let unsynced = match db::get_unsynced_sessions(conn) {
            Ok(s) => s,
            Err(e) => {
                self.sync_status = SyncStatus::Failure(e.to_string());
                return;
            }
        };

        if unsynced.is_empty() {
            self.sync_status = SyncStatus::Idle;
            self.toast = Some(Toast {
                message: "Nothing to sync".to_string(),
                style: ToastStyle::Info,
                expires_at: std::time::Instant::now() + std::time::Duration::from_secs(3),
            });
            return;
        }

        self.sync_status = SyncStatus::Syncing;

        let cfg = match crate::config::load() {
            Ok(c) => c,
            Err(e) => {
                self.sync_status = SyncStatus::Failure(e.to_string());
                return;
            }
        };

        let payload: Vec<nerdtime_core::SyncPayload> = unsynced
            .iter()
            .map(|s| nerdtime_core::SyncPayload {
                id: s.id,
                project_name: s.project_name.clone(),
                branch_name: s.branch_name.clone(),
                commit_hash: s.commit_hash.clone(),
                description: s.description.clone(),
                started_at: s.started_at,
                ended_at: s.ended_at,
                task_id: s.task_id.clone(),
                estimated_seconds: s.estimated_seconds,
                labels: s.labels.clone(),
            })
            .collect();

        let sync_url = format!("{}/sync", cfg.api_url.trim_end_matches('/'));
        let client = reqwest::blocking::Client::new();
        let mut request = client.post(&sync_url).json(&payload);
        if let Some(ref token) = cfg.token {
            request = request.bearer_auth(token);
        }

        match request.send() {
            Ok(resp) if resp.status().is_success() => {
                let count = db::mark_synced(conn).unwrap_or(0);
                self.sync_status = SyncStatus::Success(count);
                self.unsynced_count = 0;
                self.toast = Some(Toast {
                    message: format!("Synced {} sessions", count),
                    style: ToastStyle::Success,
                    expires_at: std::time::Instant::now() + std::time::Duration::from_secs(3),
                });
            }
            Ok(resp) if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 => {
                self.sync_status = SyncStatus::Failure("Auth rejected".to_string());
                self.toast = Some(Toast {
                    message: "Sync rejected — check subscription".to_string(),
                    style: ToastStyle::Error,
                    expires_at: std::time::Instant::now() + std::time::Duration::from_secs(5),
                });
            }
            Ok(resp) => {
                let msg = format!("Sync failed: {}", resp.status());
                self.sync_status = SyncStatus::Failure(msg.clone());
                self.toast = Some(Toast {
                    message: msg,
                    style: ToastStyle::Error,
                    expires_at: std::time::Instant::now() + std::time::Duration::from_secs(5),
                });
            }
            Err(e) => {
                let msg = format!("Sync error: {}", e);
                self.sync_status = SyncStatus::Failure(msg.clone());
                self.toast = Some(Toast {
                    message: msg,
                    style: ToastStyle::Error,
                    expires_at: std::time::Instant::now() + std::time::Duration::from_secs(5),
                });
            }
        }
    }
}
