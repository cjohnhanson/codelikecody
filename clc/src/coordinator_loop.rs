//! Coordinator process: runs inside a workspace, polls the coordination DB,
//! handles mechanical operations directly, invokes Claude for judgment calls.
//!
//! Started by the supervisor via `clc coordinator-run`. Communicates with
//! workers and the supervisor entirely through the coordination DB.

use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::thread;
use std::time::Duration;

use camino::Utf8Path;
use clc_sdk::agent::{Agent, AgentConfig, ClaudeCodeAgent};

use crate::config::CoordinatorScope;
use crate::coordination::Coordination;
use crate::error::Error;
use crate::git;

/// Persistent state for the coordinator's Claude session.
struct CoordinatorSession {
    agent: ClaudeCodeAgent,
    session_id: Option<String>,
    model: String,
    project_dir: std::path::PathBuf,
}

impl CoordinatorSession {
    fn new(model: &str, project_dir: &Path) -> Self {
        Self {
            agent: ClaudeCodeAgent::new(),
            session_id: None,
            model: model.to_string(),
            project_dir: project_dir.to_path_buf(),
        }
    }

    /// Invoke the Claude session with a message. Returns Claude's text response.
    /// First call starts a new session; subsequent calls resume the same session.
    fn invoke(&mut self, message: &str) -> Result<String, Error> {
        let mut cmd = if let Some(ref sid) = self.session_id {
            self.agent
                .build_resume_command(sid, &self.project_dir)
                .map_err(|e| Error::NonBlocking(format!("build resume command: {e}")))?
        } else {
            let config = AgentConfig {
                model: self.model.clone(),
                system_prompt: coordinator_system_prompt(),
                initial_prompt: String::new(),
                extra_args: vec![],
            };
            self.agent
                .build_start_command(&config, &self.project_dir)
                .map_err(|e| Error::NonBlocking(format!("build start command: {e}")))?
        };

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        let mut child = cmd
            .spawn()
            .map_err(|e| Error::NonBlocking(format!("spawn claude: {e}")))?;

        // Send the message via stdin.
        if let Some(mut stdin) = child.stdin.take() {
            let input = claude_code::protocol::InputMessage::user(message);
            let json = serde_json::to_string(&input)?;
            let _ = writeln!(stdin, "{json}");
        }

        let output = child
            .wait_with_output()
            .map_err(|e| Error::NonBlocking(format!("wait for claude: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract session ID and text response from NDJSON output.
        let mut response_text = String::new();
        for line in stdout.lines() {
            if let Ok(msg) = serde_json::from_str::<claude_code::protocol::OutputMessage>(line) {
                match msg {
                    claude_code::protocol::OutputMessage::System(ref sys) => {
                        if let Some(ref sid) = sys.session_id {
                            self.session_id = Some(sid.clone());
                        }
                    }
                    claude_code::protocol::OutputMessage::Assistant(ref a) => {
                        for block in &a.message.content {
                            if let claude_code::protocol::ContentBlock::Text { text } = block {
                                response_text.push_str(text);
                                response_text.push('\n');
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(response_text)
    }
}

fn coordinator_system_prompt() -> String {
    "You are a coordinator agent managing autonomous workers. \
     You have access to clc commands as tools:\n\n\
     - `clc permissions grant <worker-id> \"<pattern>\"` — grant a permission\n\
     - `clc permissions deny <worker-id> \"<reason>\"` — deny a permission\n\
     - `clc permissions escalate <worker-id> \"<reason>\"` — escalate to human\n\
     - `clc worker <id> check` — see worker's recent output\n\
     - `clc worker <id> send \"<message>\"` — send a message to a worker\n\
     - `clc workers` — list all workers\n\
     - `clc land <id>` — land completed work\n\n\
     When asked to handle a situation, use these tools to investigate and act. \
     Do not respond with text — take action."
        .to_string()
}

/// Run the coordinator loop. Blocks until all work is done or the process is killed.
pub fn run(
    project_dir: &Path,
    main_branch: &str,
    admin_branch: &str,
    scope: &CoordinatorScope,
    worker_perm_defaults: &[String],
    worker_perm_deny: &[String],
    poll_interval: Duration,
) -> Result<(), Error> {
    let git_state = git::detect(project_dir, main_branch, admin_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if !git_state.is_main {
        return Err(Error::NonBlocking(format!(
            "coordinator must run from the main branch (currently on '{}')",
            git_state.branch
        )));
    }

    let coord = Coordination::open(project_dir)
        .map_err(|e| Error::NonBlocking(format!("coordination DB: {e}")))?;

    // Register (or re-register on restart — ignore duplicate error).
    if coord.register_agent(&scope.id, Some("supervisor")).is_err() {
        // Already registered from a prior run — just update status.
    }
    let _ = coord.set_status(&scope.id, clc_sdk::coordination::AgentStatus::Running);

    let mut session = CoordinatorSession::new(&scope.model, project_dir);

    eprintln!("coordinator '{}' started (poll every {poll_interval:?})", scope.id);

    loop {
        match tick(project_dir, main_branch, admin_branch, scope, worker_perm_defaults, worker_perm_deny, &coord, &mut session) {
            Ok(TickResult::Continue) => {}
            Ok(TickResult::AllDone) => {
                eprintln!("coordinator '{}': all work completed", scope.id);
                let _ = coord.set_status(&scope.id, clc_sdk::coordination::AgentStatus::Completed);
                return Ok(());
            }
            Err(e) => {
                eprintln!("coordinator '{}' tick error: {e}", scope.id);
            }
        }

        thread::sleep(poll_interval);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum TickResult {
    Continue,
    AllDone,
}

fn tick(
    project_dir: &Path,
    main_branch: &str,
    admin_branch: &str,
    scope: &CoordinatorScope,
    worker_perm_defaults: &[String],
    worker_perm_deny: &[String],
    coord: &Coordination,
    session: &mut CoordinatorSession,
) -> Result<TickResult, Error> {
    // 1. Dispatch pickable tiskets up to max_workers.
    let running = coord
        .list_agents(Some(&scope.id))
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, s)| *s == clc_sdk::coordination::AgentStatus::Running)
        .count();

    if running < scope.max_workers {
        let pickable = find_undispatched(project_dir, scope, coord)?;
        let slots = scope.max_workers - running;

        for id in pickable.iter().take(slots) {
            eprintln!("coordinator '{}': dispatching '{id}'", scope.id);
            let ws_type = match scope.workspace {
                crate::config::WorkspaceType::Worktree => crate::dispatch::DispatchWorkspace::Worktree,
                crate::config::WorkspaceType::Docker => {
                    let ca = std::sync::Arc::new(
                        crate::tls::EphemeralCA::new().expect("ephemeral CA for dispatch"),
                    );
                    let api_port = std::env::var("CLC_API_PORT")
                        .ok()
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(19100);
                    // Each worker gets a unique tunnel port.
                    let tunnel_port = 19200 + (running + pickable.iter().position(|x| x == id).unwrap_or(0)) as u16;
                    crate::dispatch::DispatchWorkspace::Docker {
                        image: scope.docker_image.clone(),
                        ca: Some(ca),
                        api_port,
                        tunnel_port,
                    }
                }
            };
            match crate::dispatch::dispatch_with_workspace(
                project_dir,
                id,
                main_branch,
                admin_branch,
                &scope.model,
                worker_perm_defaults,
                worker_perm_deny,
                Some(&scope.id),
                &ws_type,
            ) {
                Ok(()) => {}
                Err(e) => eprintln!("coordinator '{}': dispatch failed for '{id}': {e}", scope.id),
            }
        }
    }

    // 2. Land completed workers.
    let agents = coord.list_agents(Some(&scope.id)).unwrap_or_default();
    for (id, status) in &agents {
        if *status == clc_sdk::coordination::AgentStatus::Completed {
            eprintln!("coordinator '{}': landing '{id}'", scope.id);

            // For Docker workspaces, import the pack from the workspace first.
            if matches!(scope.workspace, crate::config::WorkspaceType::Docker) {
                let api_port = std::env::var("CLC_API_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(19100u16);
                let api_url = format!("http://127.0.0.1:{api_port}");

                match import_workspace_pack(&api_url, id, project_dir) {
                    Ok(()) => eprintln!("coordinator '{}': imported pack from '{id}'", scope.id),
                    Err(e) => {
                        eprintln!("coordinator '{}': pack import failed for '{id}': {e}", scope.id);
                        continue;
                    }
                }
            }

            match crate::merge::merge(project_dir, id, main_branch, admin_branch) {
                Ok(()) => eprintln!("coordinator '{}': landed '{id}'", scope.id),
                Err(e) => {
                    // Merge failed — invoke Claude to resolve.
                    eprintln!("coordinator '{}': land failed for '{id}', invoking Claude", scope.id);
                    let msg = format!(
                        "Landing worker '{id}' failed with: {e}\n\
                         Investigate and resolve this. Use `clc land {id}` to retry after fixing."
                    );
                    match session.invoke(&msg) {
                        Ok(_) => {} // Claude acted via tools.
                        Err(invoke_err) => eprintln!("coordinator '{}': Claude invocation failed: {invoke_err}", scope.id),
                    }
                }
            }
        }
    }

    // 3. Resume stopped workers (unless they have a pending permission request).
    for (id, status) in &agents {
        if *status == clc_sdk::coordination::AgentStatus::Stopped
            || *status == clc_sdk::coordination::AgentStatus::Failed
        {
            if crate::permissions::pending_request(project_dir, id).is_some() {
                continue;
            }
            eprintln!("coordinator '{}': resuming '{id}'", scope.id);
            match crate::worker::resume(project_dir, id) {
                Ok(()) => {}
                Err(e) => eprintln!("coordinator '{}': resume failed for '{id}': {e}", scope.id),
            }
        }
    }

    // 4. Handle permission requests.
    let pending = coord.pending_permissions(&scope.id).unwrap_or_default();
    for msg in &pending {
        if let clc_sdk::coordination::MessageKind::PermissionRequest {
            ref tool_name,
            ref reason,
        } = msg.kind
        {
            let worker_id = &msg.from;

            // Mechanical: auto-grant if pattern matches.
            if scope.auto_grant.iter().any(|p| tool_name.contains(p)) {
                eprintln!("coordinator '{}': auto-granting '{tool_name}' for '{worker_id}'", scope.id);
                let _ = crate::permissions::grant(project_dir, worker_id, tool_name);
                continue;
            }

            // Mechanical: escalate if pattern matches.
            if scope.always_escalate.iter().any(|p| tool_name.contains(p)) {
                eprintln!("coordinator '{}': escalating '{tool_name}' for '{worker_id}'", scope.id);
                let _ = crate::permissions::escalate(project_dir, worker_id, reason);
                continue;
            }

            // Judgment: invoke Claude. Claude has clc tools and will act directly
            // (grant, deny, or escalate via tool calls).
            eprintln!(
                "coordinator '{}': invoking Claude for permission '{tool_name}' from '{worker_id}'",
                scope.id
            );
            let prompt = format!(
                "Worker '{worker_id}' is requesting permission for '{tool_name}'. \
                 Reason: {reason}\n\
                 Check the worker's context with `clc worker {worker_id} check` and \
                 handle this permission request."
            );
            match session.invoke(&prompt) {
                Ok(_) => {
                    // Claude acted via tool calls (grant/deny/escalate).
                    // If it didn't act, the request stays pending for next tick.
                }
                Err(e) => {
                    eprintln!("coordinator '{}': Claude failed ({e}), escalating", scope.id);
                    let _ = crate::permissions::escalate(project_dir, worker_id, reason);
                }
            }
        }
    }

    // 5. Check if all work is done.
    let all_agents = coord.list_agents(Some(&scope.id)).unwrap_or_default();
    let any_active = all_agents.iter().any(|(_, s)| {
        *s == clc_sdk::coordination::AgentStatus::Running
            || *s == clc_sdk::coordination::AgentStatus::Pending
            || *s == clc_sdk::coordination::AgentStatus::Stopped
            || *s == clc_sdk::coordination::AgentStatus::Failed
    });

    let pickable = find_undispatched(project_dir, scope, coord)?;

    if !any_active && pickable.is_empty() {
        return Ok(TickResult::AllDone);
    }

    Ok(TickResult::Continue)
}

/// Fetch pack from workspace via the supervisor API and import into host repo.
fn import_workspace_pack(
    api_url: &str,
    worker_id: &str,
    project_dir: &Path,
) -> Result<(), crate::error::Error> {
    use crate::error::Error;

    let url = format!("{api_url}/agents/{worker_id}/pack");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::NonBlocking(format!("tokio: {e}")))?;

    let body: serde_json::Value = rt.block_on(async {
        let client = crate::coordination_client::build_api_client()
            .map_err(|e| Error::NonBlocking(format!("http client: {e}")))?;
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::NonBlocking(format!("fetch pack: {e}")))?;
        resp.json()
            .await
            .map_err(|e| Error::NonBlocking(format!("parse pack: {e}")))
    })?;

    let pack_b64 = body["pack"]
        .as_str()
        .ok_or_else(|| Error::NonBlocking("missing pack field".into()))?;
    let pack_data = crate::base64_decode(pack_b64)
        .map_err(|e| Error::NonBlocking(format!("decode pack: {e}")))?;

    let refs: Vec<(String, String)> = body["refs"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|r| {
            let arr = r.as_array()?;
            Some((
                arr.first()?.as_str()?.to_string(),
                arr.get(1)?.as_str()?.to_string(),
            ))
        })
        .collect();

    crate::git_pack::import_pack(&pack_data, &refs, project_dir)?;

    Ok(())
}

fn find_undispatched(
    project_dir: &Path,
    scope: &CoordinatorScope,
    coord: &Coordination,
) -> Result<Vec<String>, Error> {
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let repo =
        tisket::Repo::open(utf8_dir).map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    let issues = repo
        .list_issues(scope.project.as_deref(), None, None, false, &[])
        .map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    let dispatched: Vec<String> = coord
        .list_agents(Some(&scope.id))
        .unwrap_or_default()
        .into_iter()
        .map(|(id, _)| id)
        .collect();

    let pickable: Vec<String> = issues
        .into_iter()
        .filter(|i| i.frontmatter.status.is_pickable())
        .filter(|i| {
            i.frontmatter.depends_on.iter().all(|dep_id| {
                repo.find_issue(dep_id)
                    .map(|dep| dep.closed)
                    .unwrap_or(false)
            })
        })
        .filter(|i| {
            scope
                .label
                .as_deref()
                .is_none_or(|l| i.frontmatter.labels.iter().any(|il| il == l))
        })
        .filter(|i| {
            scope
                .exclude_label
                .as_deref()
                .is_none_or(|l| !i.frontmatter.labels.iter().any(|il| il == l))
        })
        .map(|i| i.id)
        .filter(|id| !dispatched.contains(id))
        .collect();

    Ok(pickable)
}
