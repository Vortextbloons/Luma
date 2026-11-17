use super::{
    document::Config,
    plugin::{self, Trigger},
    Store,
};
use crate::extension_spike;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::{
    collections::HashSet,
    ffi::OsStr,
    io::ErrorKind,
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicI32, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, WebviewWindow};

const SURFACE_ID: &str = "com.luma.acceptance.surface";
const SERVICE_ID: &str = "com.luma.acceptance.service";
const CRASH_ID: &str = "com.luma.acceptance.crash";
const READY_LIMIT: Duration = Duration::from_secs(5);
const RESTART_LIMIT: u32 = 3;

pub fn run() -> i32 {
    let store_root =
        std::env::temp_dir().join(format!("luma-runtime-acceptance-{}", std::process::id()));
    let identifier = format!("com.luma.acceptance.p{}", std::process::id());
    if let Err(error) = std::fs::create_dir(&store_root)
        .and_then(|_| std::fs::create_dir(store_root.join("plugins")))
    {
        eprintln!("FAIL: could not create the acceptance store: {error}");
        return 2;
    }
    let cleanup = Arc::new(Mutex::new(vec![store_root.clone()]));
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    context.config_mut().identifier = identifier.clone();

    let exit_code = Arc::new(AtomicI32::new(2));
    let setup_root = store_root.clone();
    let setup_identifier = identifier.clone();
    let setup_code = exit_code.clone();
    let setup_cleanup = cleanup.clone();
    let mut builder = tauri::Builder::default();
    for scheme in extension_spike::schemes() {
        builder = builder.register_uri_scheme_protocol(scheme, extension_spike::protocol);
    }
    let app = builder
        .manage(extension_spike::Supervisor::default())
        .manage(crate::remote::server::Remote::default())
        .invoke_handler(extension_spike::guard(tauri::generate_handler![
            acceptance_core_probe
        ]))
        .setup(move |app| {
            let data = app
                .path()
                .app_data_dir()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            if data.file_name() != Some(OsStr::new(&setup_identifier)) {
                return Err(std::io::Error::other(format!(
                    "app data directory {} does not match acceptance identifier {setup_identifier}",
                    data.display()
                ))
                .into());
            }
            setup_cleanup.lock().unwrap().push(data.clone());
            let store = Store::from_connection_with_root(
                Connection::open(setup_root.join("customization.sqlite3"))
                    .map_err(|error| std::io::Error::other(error.to_string()))?,
                Some(setup_root.join("plugins")),
            )
            .map_err(|error| std::io::Error::other(error.message))?;
            store
                .initialize(Config::default())
                .map_err(|error| std::io::Error::other(error.message))?;
            app.manage(store);
            println!("acceptance store: {}", setup_root.display());
            println!(
                "acceptance identifier: {setup_identifier} (user data {})",
                data.display()
            );
            let handle = app.handle().clone();
            let code = setup_code.clone();
            let thread_root = setup_root.clone();
            let thread_data = data.clone();
            std::thread::spawn(move || {
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    harness(&handle, &thread_root, &thread_data)
                }));
                let value = match outcome {
                    Ok(Ok(())) => {
                        println!("PASS: live plugin runtime acceptance");
                        0
                    }
                    Ok(Err(error)) => {
                        println!("FAIL: {error}");
                        1
                    }
                    Err(_) => {
                        println!("FAIL: acceptance harness panicked");
                        1
                    }
                };
                schedule_cleanup(&thread_root, &thread_data);
                code.store(value, Ordering::SeqCst);
                handle.exit(value);
            });
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_secs(180));
                eprintln!("FAIL: acceptance harness timed out");
                std::process::exit(2);
            });
            Ok(())
        })
        .build(context)
        .expect("acceptance app should build");
    let run_cleanup = cleanup.clone();
    let _code = app.run_return(|_, _| {});
    clean_paths(&run_cleanup.lock().unwrap());
    exit_code.load(Ordering::SeqCst)
}

fn schedule_cleanup(store_root: &Path, user_data: &Path) {
    let script = format!(
        "Start-Sleep -Seconds 3; Remove-Item -LiteralPath '{}' -Recurse -Force -ErrorAction SilentlyContinue; Remove-Item -LiteralPath '{}' -Recurse -Force -ErrorAction SilentlyContinue",
        store_root.display().to_string().replace(char::from(39), "''"),
        user_data.display().to_string().replace(char::from(39), "''")
    );
    let _ = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn clean_paths(paths: &[PathBuf]) {
    std::thread::sleep(Duration::from_secs(1));
    for attempt in 0..40 {
        let mut remaining = Vec::new();
        for path in paths {
            if !path.exists() {
                continue;
            }
            if std::fs::remove_dir_all(path).is_err() {
                remaining.push(path.clone());
            }
        }
        if remaining.is_empty() {
            return;
        }
        if attempt == 39 {
            for path in remaining {
                eprintln!("NOTE: acceptance data left at {}", path.display());
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

#[tauri::command]
fn acceptance_core_probe() -> &'static str {
    "core-command-reached"
}

fn harness(app: &AppHandle, store_root: &Path, user_data: &Path) -> Result<(), String> {
    let store = app.state::<Store>();
    let store = store.inner();
    require(
        app.get_webview_window("main").is_none(),
        "the harness unexpectedly created a launcher window",
    )?;
    require(
        extension_spike::active(app) == false,
        "a package surface was active before any runtime started",
    )?;
    println!("PASS: acceptance process has no launcher window and no active package surface");

    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("could not bind the CSP probe listener: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("could not configure the CSP probe listener: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("CSP probe listener has no address: {error}"))?
        .port();

    stage(
        store,
        surface_manifest(port),
        &[
            ("ui/index.html", surface_index()),
            ("ui/probe.js", surface_probe()),
            ("ui/probe.css", surface_style()),
        ],
    )?;
    stage(
        store,
        service_manifest(),
        &[
            ("worker.html", service_page()),
            ("worker.js", service_script()),
        ],
    )?;
    stage(store, crash_manifest(), &[("crash.html", crash_page())])?;
    pass("staged three executable packages through the real customization store");
    require(
        plugin::runtime_package(store, SURFACE_ID).unwrap_err().code == "plugin_disabled",
        "a staged executable package was considered runnable before its grant",
    )?;
    pass("staged packages are not runnable before an approved grant");

    grant(
        store,
        SURFACE_ID,
        &["settings.read"],
        &[],
        Trigger::LauncherOpen,
    )?;
    grant(store, SERVICE_ID, &[], &[], Trigger::Startup)?;
    grant(store, CRASH_ID, &[], &[], Trigger::Startup)?;
    let state = plugin::state_for_app(app).map_err(|error| error.message)?;
    let surface = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == SURFACE_ID)
        .ok_or_else(|| "surface package is missing after its grant".to_string())?;
    require(
        surface.enabled,
        "surface package is not enabled after its grant",
    )?;
    require(
        surface.surface_openable,
        "isolated-ui package is not openable after its grant",
    )?;
    require(
        surface.granted_permissions == vec!["settings.read".to_string()],
        format!(
            "surface grant is not exact: {:?}",
            surface.granted_permissions
        ),
    )?;
    require(
        surface.trigger.as_deref() == Some("launcher-open"),
        "surface trigger was not stored",
    )?;
    let service = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == SERVICE_ID)
        .ok_or_else(|| "service package is missing after its grant".to_string())?;
    require(
        service.enabled,
        "service package is not enabled after its grant",
    )?;
    require(
        !service.surface_openable,
        "background package advertised an openable surface",
    )?;
    require(
        service.trigger.as_deref() == Some("startup"),
        "service trigger was not stored",
    )?;
    pass("approved exact grants with plugin::command_enable_runtime");

    let package = plugin::runtime_package(store, SURFACE_ID).map_err(|error| error.message)?;
    require(
        package.allows(plugin::Permission::SettingsRead),
        "granted settings.read is missing from the runtime package",
    )?;
    require(
        !package.allows(plugin::Permission::PlaybackRead),
        "ungranted playback.read leaked into the runtime package",
    )?;
    require(
        package.network.is_empty(),
        "an undeclared network origin leaked into the runtime package",
    )?;
    pass("runtime package exposes only the exact granted capability");

    let _ = std::fs::remove_dir_all(user_data.join("extension-runtime"));
    std::thread::sleep(Duration::from_millis(800));

    let baseline = measure("BASELINE (no package runtime)")?;
    require(
        baseline.webview_count == 0,
        format!(
            "baseline unexpectedly had {} WebView2 processes",
            baseline.webview_count
        ),
    )?;

    let startup_started = Instant::now();
    let startup_before = sample_processes()?;
    let service_start = Instant::now();
    extension_spike::refresh(app, SERVICE_ID);
    let _service_status = wait_running(app, SERVICE_ID, Duration::from_secs(10))?;
    let service_ready = service_start.elapsed();
    let startup_after = sample_processes()?;
    let startup_interval = startup_started.elapsed();
    let startup_seconds = startup_interval.as_secs_f64().max(0.001);
    println!(
        "BACKGROUND STARTUP (ready {:.0}ms, sample {:.1}s): host {:.2}% CPU, {:.1} MiB private; WebView2 {} procs {:.2}% CPU, {:.1} MiB private",
        service_ready.as_secs_f64() * 1000.0,
        startup_interval.as_secs_f64(),
        ((startup_after.host_cpu - startup_before.host_cpu).max(0.0) / startup_seconds) * 100.0,
        mib(startup_after.host_private),
        startup_after.webview_count,
        ((startup_after.webview_cpu - startup_before.webview_cpu).max(0.0) / startup_seconds)
            * 100.0,
        mib(startup_after.webview_private)
    );
    require(
        service_ready < READY_LIMIT,
        format!("background package readiness exceeded the 5s limit: {service_ready:?}"),
    )?;
    let service_window = wait_runtime_window(app, "Acceptance Service", Duration::from_secs(5))?;
    let service_marker = wait_marker(
        &service_window,
        "window.__lumaService",
        |value| value["ready"].as_bool() == Some(true),
        Duration::from_secs(5),
    )?;
    println!(
        "PASS: background package became running in {:.0}ms and acknowledged surface.ready through the bridge (first beat {})",
        service_ready.as_secs_f64() * 1000.0,
        service_marker["beat"]
    );

    std::thread::sleep(Duration::from_millis(800));
    let beat_before = eval_json(&service_window, "window.__lumaService.beat")?
        .as_u64()
        .unwrap_or(0);
    let service_idle = measure("BACKGROUND PACKAGE IDLE")?;
    let beat_after = eval_json(&service_window, "window.__lumaService.beat")?
        .as_u64()
        .unwrap_or(0);
    require(
        beat_after > beat_before,
        format!(
            "background package stopped executing during the idle interval ({beat_before} -> {beat_after})"
        ),
    )?;
    require(
        service_idle.webview_count > 0,
        "background package had no WebView2 process during its idle interval",
    )?;
    println!(
        "PASS: background package executed {} beats during the idle interval ({} -> {})",
        beat_after - beat_before,
        beat_before,
        beat_after
    );
    println!(
        "NOTE: Luma defines no CPU or private-memory budget; values above are measured samples, not invented thresholds."
    );

    let surface_start = Instant::now();
    extension_spike::open(app, SURFACE_ID).map_err(|error| format!("open surface: {error}"))?;
    let _surface_status = wait_running(app, SURFACE_ID, Duration::from_secs(10))?;
    let surface_ready = surface_start.elapsed();
    require(
        surface_ready < READY_LIMIT,
        format!("isolated-ui readiness exceeded the 5s limit: {surface_ready:?}"),
    )?;
    let surface_window = wait_runtime_window(app, "Acceptance Surface", Duration::from_secs(5))?;
    let marker = wait_marker(
        &surface_window,
        "window.__lumaAcceptance",
        |value| value["done"].as_bool() == Some(true),
        Duration::from_secs(10),
    )?;
    println!(
        "PASS: isolated-ui surface became running in {:.0}ms and acknowledged surface.ready through the bridge",
        surface_ready.as_secs_f64() * 1000.0
    );

    assert_surface_marker(&marker, port)?;
    let session_origin = format!(
        "{}/{}",
        marker["origin"].as_str().unwrap_or_default(),
        marker["token"].as_str().unwrap_or_default()
    );
    require_runtime_origin(app, SURFACE_ID, &session_origin)?;
    let leaks = drain_connections(&listener);
    require(
        leaks == 0,
        format!("CSP probe established {leaks} TCP connections to 127.0.0.1:{port}"),
    )?;
    pass("package JavaScript could not open a network connection outside the CSP allowlist");

    let surface_token = marker["token"]
        .as_str()
        .ok_or_else(|| "surface marker has no origin token".to_string())?
        .to_string();
    require(!surface_token.is_empty(), "surface origin token was empty")?;

    extension_spike::launcher_hidden(app);
    wait_gone(app, SURFACE_ID, Duration::from_secs(5))?;
    require(
        service_running(app)?,
        "launcher hide stopped the startup background service",
    )?;
    wait_runtime_window_gone(app, "Acceptance Surface", Duration::from_secs(5))?;
    pass("launcher hide stopped the launcher-open surface and preserved the startup background service");

    save_settings(
        store,
        SURFACE_ID,
        json!({ "foreignOrigin": session_origin }),
    )?;
    extension_spike::open(app, SURFACE_ID).map_err(|error| format!("reopen surface: {error}"))?;
    wait_running(app, SURFACE_ID, Duration::from_secs(10))?;
    let second_window = wait_runtime_window(app, "Acceptance Surface", Duration::from_secs(5))?;
    let second = wait_marker(
        &second_window,
        "window.__lumaAcceptance",
        |value| value["done"].as_bool() == Some(true),
        Duration::from_secs(10),
    )?;
    let second_token = second["token"]
        .as_str()
        .ok_or_else(|| "second surface marker has no origin token".to_string())?;
    require(
        second_token != surface_token,
        "a restarted surface reused its previous session origin",
    )?;
    let second_origin = format!(
        "{}/{}",
        second["origin"].as_str().unwrap_or_default(),
        second_token
    );
    require_runtime_origin(app, SURFACE_ID, &second_origin)?;
    require(
        second_origin != session_origin,
        "runtime supervisor still reported the previous session origin",
    )?;
    let foreign = step(&second, "foreign")?;
    require(
        foreign["ok"].as_bool() == Some(true),
        format!("foreign-session probe failed: {foreign}"),
    )?;
    require(
        foreign["value"]["blocked"].as_bool() == Some(true),
        format!("navigation to the previous session origin was not blocked: {foreign}"),
    )?;
    require(
        foreign["value"]["target"].as_str() == Some(format!("{session_origin}/").as_str()),
        "the foreign-session probe did not target the previous full origin",
    )?;
    pass("a new session used a new origin and navigation to the previous session token was denied");

    extension_spike::refresh(app, SERVICE_ID);
    wait_running(app, SERVICE_ID, Duration::from_secs(10))?;
    save_settings(store, SERVICE_ID, json!({ "marker": "keep-me" }))?;
    let current = revision(store)?;
    plugin::command_disable(store, SERVICE_ID, current)
        .map_err(|error| format!("disable service: {}: {}", error.code, error.message))?;
    extension_spike::invalidate(app, SERVICE_ID);
    wait_gone(app, SERVICE_ID, Duration::from_secs(5))?;
    let state = plugin::state_for_app(app).map_err(|error| error.message)?;
    let service = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == SERVICE_ID)
        .ok_or_else(|| "service package disappeared on disable".to_string())?;
    require(
        !service.enabled,
        "service package stayed enabled after disable",
    )?;
    require(
        service.granted_permissions.is_empty() && service.trigger.is_none(),
        "disable did not clear the runtime grant",
    )?;
    require(
        plugin::runtime_package(store, SERVICE_ID).unwrap_err().code == "plugin_disabled",
        "a disabled package was still exposed as runnable",
    )?;
    wait_runtime_window_gone(app, "Acceptance Service", Duration::from_secs(5))?;
    pass("disable stopped the background package, cleared its grant, and kept it installed");

    let current = revision(store)?;
    plugin::command_remove(store, SERVICE_ID, current, false)
        .map_err(|error| format!("remove service: {}: {}", error.code, error.message))?;
    require(
        record(app, SERVICE_ID)?.is_none(),
        "service package row survived removal",
    )?;
    require(
        sql_count(
            store,
            "SELECT COUNT(*) FROM plugin_settings WHERE plugin_id = ?1",
            SERVICE_ID,
        )? == 1,
        "default removal deleted the package's saved settings",
    )?;
    require(
        !store_root.join("plugins").join(SERVICE_ID).exists(),
        "removal left the installed package files behind",
    )?;
    require(
        user_data
            .join("extension-runtime")
            .join(SERVICE_ID)
            .exists(),
        "default removal deleted the package's runtime profile",
    )?;
    pass("remove with deleteData=false removed code but preserved settings and runtime profile");

    save_settings(store, CRASH_ID, json!({ "marker": "delete-me" }))?;
    extension_spike::refresh(app, CRASH_ID);
    let failures = wait_crash_cycle(app, store_root, CRASH_ID, Duration::from_secs(45))?;
    for restarts in 0..=RESTART_LIMIT {
        require(
            failures.contains(&("starting".to_string(), restarts)),
            format!("bounded restart never observed starting attempt {restarts}: {failures:?}"),
        )?;
    }
    let failed = extension_spike::runtime_status(app, CRASH_ID)
        .ok_or_else(|| "failed runtime status disappeared before inspection".to_string())?;
    require(
        failed.state == "failed" && failed.restarts == RESTART_LIMIT,
        format!("runtime did not stop at the restart limit: {failed:?}"),
    )?;
    require(
        failed
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("stopped restarting it after 3 attempts")),
        format!("restart limit reason was not explicit: {failed:?}"),
    )?;
    let state = plugin::state_for_app(app).map_err(|error| error.message)?;
    let crash = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == CRASH_ID)
        .ok_or_else(|| "crash package disappeared after repeated failures".to_string())?;
    require(
        !crash.enabled && crash.granted_permissions.is_empty() && crash.trigger.is_none(),
        "repeated runtime failures did not disable the package and clear its grant",
    )?;
    require(
        crash
            .disabled_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("stopped restarting it after 3 attempts")),
        format!(
            "disabled reason did not explain the restart limit: {:?}",
            crash.disabled_reason
        ),
    )?;
    wait_runtime_window_gone(app, "Acceptance Failure", Duration::from_secs(5))?;
    pass("bounded failure recovery restarted 3 times, stopped, and disabled the package");

    let current = revision(store)?;
    plugin::command_remove(store, CRASH_ID, current, true)
        .map_err(|error| format!("remove crash package: {}: {}", error.code, error.message))?;
    tauri::async_runtime::block_on(extension_spike::remove_data(app, CRASH_ID))
        .map_err(|error| format!("remove crash runtime data: {error}"))?;
    wait_gone(app, CRASH_ID, Duration::from_secs(5))?;
    require(
        sql_count(
            store,
            "SELECT COUNT(*) FROM plugin_settings WHERE plugin_id = ?1",
            CRASH_ID,
        )? == 0,
        "deleteData=true kept the package's saved settings",
    )?;
    require(
        !user_data.join("extension-runtime").join(CRASH_ID).exists(),
        "deleteData=true kept the package's runtime profile",
    )?;
    require(
        !store_root.join("plugins").join(CRASH_ID).exists(),
        "deleteData=true kept the installed package files",
    )?;
    pass("remove with deleteData=true deleted settings, runtime profile, and package files");

    extension_spike::shutdown(app);
    wait_gone(app, SERVICE_ID, Duration::from_secs(5))?;
    wait_gone(app, SURFACE_ID, Duration::from_secs(5))?;
    wait_gone(app, CRASH_ID, Duration::from_secs(5))?;
    wait_no_runtime_windows(app, Duration::from_secs(5))?;
    pass("exit cleanup destroyed every package runtime and window");

    println!(
        "SUMMARY: baseline and background measurements use the same 5s interval. Startup readiness limits: {}s. Restart limit: {} attempts within the runtime's 600s window.",
        READY_LIMIT.as_secs(),
        RESTART_LIMIT
    );
    Ok(())
}

fn stage(store: &Store, manifest: Value, files: &[(&str, &str)]) -> Result<(), String> {
    let files: Vec<Value> = files
        .iter()
        .map(|(path, text)| json!({ "path": path, "text": text }))
        .collect();
    plugin::stage_submission(
        store,
        &json!({ "manifest": manifest, "files": files }),
        "acceptance",
    )
    .map(|_| ())
    .map_err(|error| format!("stage package: {}: {}", error.code, error.message))
}

fn surface_manifest(port: u16) -> Value {
    json!({
        "id": SURFACE_ID,
        "name": "Acceptance Surface",
        "version": "1.0.0",
        "apiVersion": 2,
        "description": "Live runtime acceptance fixture.",
        "author": "Luma tests",
        "runtime": { "kind": "isolated-ui", "entry": "ui/index.html" },
        "permissions": ["settings.read", "playback.read"],
        "settings": [
            {
                "key": "probePort",
                "label": "Probe port",
                "type": "string",
                "default": port.to_string(),
                "minLength": 1,
                "maxLength": 8
            },
            {
                "key": "foreignOrigin",
                "label": "Foreign origin",
                "type": "string",
                "default": "",
                "maxLength": 256
            }
        ],
        "assets": ["ui/probe.js", "ui/probe.css"]
    })
}

fn service_manifest() -> Value {
    json!({
        "id": SERVICE_ID,
        "name": "Acceptance Service",
        "version": "1.0.0",
        "apiVersion": 2,
        "description": "Live background runtime acceptance fixture.",
        "author": "Luma tests",
        "runtime": { "kind": "background", "entry": "worker.html" },
        "permissions": [],
        "settings": [
            {
                "key": "marker",
                "label": "Marker",
                "type": "string",
                "default": "fixture",
                "maxLength": 32
            }
        ],
        "assets": ["worker.js"]
    })
}

fn crash_manifest() -> Value {
    json!({
        "id": CRASH_ID,
        "name": "Acceptance Failure",
        "version": "1.0.0",
        "apiVersion": 2,
        "description": "Live bounded failure acceptance fixture.",
        "author": "Luma tests",
        "runtime": { "kind": "background", "entry": "crash.html" },
        "permissions": [],
        "settings": [
            {
                "key": "marker",
                "label": "Marker",
                "type": "string",
                "default": "fixture",
                "maxLength": 32
            }
        ]
    })
}

fn surface_index() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/plugin-runtime/surface/index.html"
    ))
}

fn surface_probe() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/plugin-runtime/surface/probe.js"
    ))
}

fn surface_style() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/plugin-runtime/surface/probe.css"
    ))
}

fn service_page() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/plugin-runtime/worker.html"
    ))
}

fn service_script() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/plugin-runtime/worker.js"
    ))
}

fn crash_page() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/plugin-runtime/crash.html"
    ))
}

fn grant(
    store: &Store,
    plugin_id: &str,
    permissions: &[&str],
    network: &[&str],
    trigger: Trigger,
) -> Result<(), String> {
    let revision = revision(store)?;
    plugin::command_enable_runtime(
        store,
        plugin_id,
        revision,
        permissions.iter().map(|value| value.to_string()).collect(),
        network.iter().map(|value| value.to_string()).collect(),
        trigger,
    )
    .map_err(|error| format!("grant {plugin_id}: {}: {}", error.code, error.message))
}

fn revision(store: &Store) -> Result<u64, String> {
    store
        .read()
        .map_err(|error| error.message)?
        .map(|document| document.revision)
        .ok_or_else(|| "customization store is not initialized".to_string())
}

fn save_settings(store: &Store, plugin_id: &str, values: Value) -> Result<(), String> {
    let revision = revision(store)?;
    let values: Map<String, Value> = values
        .as_object()
        .cloned()
        .ok_or_else(|| "settings helper requires an object".to_string())?;
    plugin::command_save_settings(store, plugin_id, revision, values).map_err(|error| {
        format!(
            "save {plugin_id} settings: {}: {}",
            error.code, error.message
        )
    })
}

fn record(app: &AppHandle, plugin_id: &str) -> Result<Option<plugin::PluginRecord>, String> {
    Ok(plugin::state_for_app(app)
        .map_err(|error| error.message)?
        .plugins
        .into_iter()
        .find(|plugin| plugin.id == plugin_id))
}

fn sql_count(store: &Store, query: &str, plugin_id: &str) -> Result<i64, String> {
    let connection = store.connection().map_err(|error| error.message)?;
    connection
        .query_row(query, [plugin_id], |row| row.get(0))
        .map_err(|error| error.to_string())
}

fn wait_running(
    app: &AppHandle,
    plugin_id: &str,
    timeout: Duration,
) -> Result<plugin::RuntimeStatus, String> {
    let deadline = Instant::now() + timeout;
    let mut last;
    loop {
        last = extension_spike::runtime_status(app, plugin_id);
        if let Some(status) = &last {
            if status.state == "running" {
                return Ok(status.clone());
            }
            if status.state == "failed" {
                return Err(format!(
                    "runtime {plugin_id} failed instead of becoming ready: {status:?}"
                ));
            }
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "runtime {plugin_id} did not become ready within {timeout:?}: {last:?}"
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn wait_gone(app: &AppHandle, plugin_id: &str, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while extension_spike::runtime_status(app, plugin_id).is_some() {
        if Instant::now() >= deadline {
            return Err(format!(
                "runtime {plugin_id} was still supervised after {timeout:?}"
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

fn wait_crash_cycle(
    app: &AppHandle,
    store_root: &Path,
    plugin_id: &str,
    timeout: Duration,
) -> Result<Vec<(String, u32)>, String> {
    let deadline = Instant::now() + timeout;
    let mut transitions: Vec<(String, u32)> = Vec::new();
    loop {
        if let Some(status) = extension_spike::runtime_status(app, plugin_id) {
            let entry = (status.state.clone(), status.restarts);
            if transitions.last() != Some(&entry) {
                println!(
                    "crash runtime transition: state={} restarts={} reason={:?}",
                    status.state, status.restarts, status.reason
                );
                transitions.push(entry);
            }
            if status.state == "failed" {
                if let Some((enabled, reason)) = crash_record(store_root, plugin_id)? {
                    if !enabled {
                        println!("crash package disabled in store: reason={reason:?}");
                        return Ok(transitions);
                    }
                }
            }
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "runtime {plugin_id} did not settle at its restart limit within {timeout:?}: {transitions:?}"
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn crash_record(
    store_root: &Path,
    plugin_id: &str,
) -> Result<Option<(bool, Option<String>)>, String> {
    let connection = Connection::open(store_root.join("customization.sqlite3"))
        .map_err(|error| format!("open acceptance database for crash check: {error}"))?;
    connection
        .busy_timeout(Duration::from_millis(500))
        .map_err(|error| error.to_string())?;
    connection
        .query_row(
            "SELECT enabled, disabled_reason FROM plugins WHERE id = ?1",
            [plugin_id],
            |row| Ok((row.get::<_, i64>(0)? != 0, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn require_runtime_origin(app: &AppHandle, plugin_id: &str, expected: &str) -> Result<(), String> {
    let actual = extension_spike::runtime_origin(app, plugin_id)
        .ok_or_else(|| format!("runtime {plugin_id} has no supervised origin"))?;
    require(
        actual == expected,
        format!("supervised origin {actual} does not match the session origin {expected}"),
    )
}

fn service_running(app: &AppHandle) -> Result<bool, String> {
    Ok(extension_spike::runtime_status(app, SERVICE_ID)
        .is_some_and(|status| status.state == "running"))
}

fn runtime_window(app: &AppHandle, name: &str) -> Option<WebviewWindow> {
    app.webview_windows().into_values().find(|window| {
        window
            .title()
            .map(|title| title.starts_with(name) && title.ends_with(" · Luma"))
            .unwrap_or(false)
    })
}

fn wait_runtime_window(
    app: &AppHandle,
    name: &str,
    timeout: Duration,
) -> Result<WebviewWindow, String> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(window) = runtime_window(app, name) {
            return Ok(window);
        }
        if Instant::now() >= deadline {
            return Err(format!("no runtime window titled {name} appeared"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn wait_runtime_window_gone(app: &AppHandle, name: &str, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while runtime_window(app, name).is_some() {
        if Instant::now() >= deadline {
            return Err(format!("runtime window titled {name} survived cleanup"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

fn wait_no_runtime_windows(app: &AppHandle, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining: Vec<String> = app
            .webview_windows()
            .into_keys()
            .filter(|label| label.starts_with("ext-"))
            .collect();
        if remaining.is_empty() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("runtime windows survived shutdown: {remaining:?}"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn eval_json(window: &WebviewWindow, expression: &str) -> Result<Value, String> {
    let script = format!("JSON.stringify({expression} ?? null)");
    let (sender, receiver) = mpsc::channel();
    window
        .eval_with_callback(script, move |value| {
            let _ = sender.send(value);
        })
        .map_err(|error| format!("eval failed: {error}"))?;
    let raw = receiver
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| format!("webview did not answer {expression}"))?;
    let serialized: String = serde_json::from_str(&raw)
        .map_err(|error| format!("eval result was not a JSON string ({raw}): {error}"))?;
    serde_json::from_str(&serialized)
        .map_err(|error| format!("eval payload was not JSON ({serialized}): {error}"))
}

fn wait_marker(
    window: &WebviewWindow,
    expression: &str,
    ready: impl Fn(&Value) -> bool,
    timeout: Duration,
) -> Result<Value, String> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(value) = eval_json(window, expression) {
            if ready(&value) {
                return Ok(value);
            }
        }
        if Instant::now() >= deadline {
            return Err(format!("webview marker {expression} never completed"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn step<'a>(marker: &'a Value, name: &str) -> Result<&'a Value, String> {
    marker["steps"]
        .as_array()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step["name"].as_str() == Some(name))
        })
        .ok_or_else(|| format!("surface marker is missing the {name} probe: {marker}"))
}

fn assert_surface_marker(marker: &Value, port: u16) -> Result<(), String> {
    let ready = step(marker, "ready")?;
    require(
        ready["ok"].as_bool() == Some(true) && ready["value"]["ready"].as_bool() == Some(true),
        format!("surface.ready was not acknowledged: {ready}"),
    )?;
    let token = marker["token"].as_str().unwrap_or_default();
    require(!token.is_empty(), "surface origin token was empty")?;
    let origin = marker["origin"].as_str().unwrap_or_default();
    let slot = origin
        .strip_prefix("http://luma-extension.s")
        .and_then(|rest| rest.strip_suffix(".localhost"))
        .filter(|slot| !slot.is_empty() && slot.bytes().all(|byte| byte.is_ascii_digit()));
    require(
        slot.is_some(),
        format!("unexpected per-session package origin: {origin}"),
    )?;
    let path = marker["path"].as_str().unwrap_or_default();
    require(
        path == format!("/{token}/ui/index.html"),
        format!("unexpected package path: {path}"),
    )?;
    let session_origin = format!("{origin}/{token}");
    let identity = step(marker, "identity")?;
    require(
        identity["ok"].as_bool() == Some(true),
        format!("identity bridge call failed: {identity}"),
    )?;
    let value = &identity["value"];
    require(
        value["id"].as_str() == Some(SURFACE_ID),
        format!("identity exposed the wrong package id: {value}"),
    )?;
    require(
        value["runtime"].as_str() == Some("isolated-ui"),
        format!("identity exposed the wrong runtime: {value}"),
    )?;
    require(
        value["surface"].as_str() == Some("desktop"),
        format!("identity did not describe an openable surface: {value}"),
    )?;
    require(
        value["origin"].as_str() == Some(session_origin.as_str()),
        format!("identity origin did not match the session origin: {value}"),
    )?;
    let settings = step(marker, "settings")?;
    require(
        settings["ok"].as_bool() == Some(true),
        format!("settings.read was denied even though it was granted: {settings}"),
    )?;
    require(
        settings["value"]["values"]["probePort"].as_str() == Some(port.to_string().as_str()),
        format!("settings.read did not return the staged value: {settings}"),
    )?;
    let permission = step(marker, "permission")?;
    require(
        permission["ok"].as_bool() == Some(false),
        format!("ungranted playback.read unexpectedly succeeded: {permission}"),
    )?;
    require(
        permission["error"]
            .as_str()
            .is_some_and(|error| error.contains("playback.read")),
        format!("ungranted playback.read was not denied by permission: {permission}"),
    )?;
    let core = step(marker, "core")?;
    require(
        core["ok"].as_bool() == Some(false),
        format!("a core command was reachable from the package webview: {core}"),
    )?;
    require(
        core["error"].as_str() == Some("Extension bridge command denied."),
        format!("core command denial was not explicit: {core}"),
    )?;
    let csp = step(marker, "csp")?;
    require(
        csp["ok"].as_bool() == Some(true),
        format!("CSP probe did not complete: {csp}"),
    )?;
    require(
        csp["value"]["failed"].as_bool() == Some(true),
        format!("network fetch outside the CSP allowlist did not fail: {csp}"),
    )?;
    println!(
        "CSP probe evidence: fetch failed={} securitypolicyviolation={}",
        csp["value"]["failed"], csp["value"]["violation"]
    );
    Ok(())
}

fn drain_connections(listener: &TcpListener) -> usize {
    let mut count = 0;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                count += 1;
                drop(stream);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
    count
}

#[derive(Clone, Debug, Default)]
struct Sample {
    host_cpu: f64,
    host_private: u64,
    webview_cpu: f64,
    webview_private: u64,
    webview_count: usize,
}

struct Interval {
    label: String,
    elapsed: Duration,
    host_cpu_percent: f64,
    webview_cpu_percent: f64,
    host_private: u64,
    webview_private: u64,
    webview_count: usize,
}

fn measure(label: &str) -> Result<Interval, String> {
    let first = sample_processes()?;
    let started = Instant::now();
    std::thread::sleep(Duration::from_secs(5));
    let second = sample_processes()?;
    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64().max(0.001);
    let interval = Interval {
        label: label.to_string(),
        elapsed,
        host_cpu_percent: ((second.host_cpu - first.host_cpu).max(0.0) / seconds) * 100.0,
        webview_cpu_percent: ((second.webview_cpu - first.webview_cpu).max(0.0) / seconds) * 100.0,
        host_private: second.host_private,
        webview_private: second.webview_private,
        webview_count: second.webview_count,
    };
    println!(
        "{} ({:.1}s): host {:.2}% CPU, {:.1} MiB private; WebView2 {} procs {:.2}% CPU, {:.1} MiB private",
        interval.label,
        interval.elapsed.as_secs_f64(),
        interval.host_cpu_percent,
        mib(interval.host_private),
        interval.webview_count,
        interval.webview_cpu_percent,
        mib(interval.webview_private)
    );
    Ok(interval)
}

fn mib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn sample_processes() -> Result<Sample, String> {
    let script = r#"$ErrorActionPreference='SilentlyContinue';$all=Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'plugin_runtime.exe' -or $_.Name -eq 'msedgewebview2.exe' -or $_.Name -eq 'crashpad_handler.exe' };$rows=foreach($p in $all){$x=Get-Process -Id $p.ProcessId;if($null -ne $x){$cpu=0.0;if($null -ne $x.CPU){$cpu=[double]$x.CPU};[pscustomobject]@{id=[int]$p.ProcessId;parent=[int]$p.ParentProcessId;name=[string]$p.Name;cpu=$cpu;private=[double]$x.PrivateMemorySize64}}};ConvertTo-Json -InputObject @($rows) -Compress"#;
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|error| format!("could not run process inspection: {error}"))?;
    if !output.status.success() && output.stdout.is_empty() {
        return Err(format!(
            "process inspection failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let value: Value = serde_json::from_str(text.trim())
        .map_err(|error| format!("process inspection returned invalid JSON ({text}): {error}"))?;
    let entries: Vec<Value> = match value {
        Value::Array(items) => items,
        Value::Object(_) => vec![value],
        Value::Null => Vec::new(),
        _ => return Err(format!("unexpected process inspection payload: {value}")),
    };
    let mut processes = Vec::new();
    for entry in entries {
        processes.push(Process {
            id: entry["id"].as_u64().unwrap_or(0) as u32,
            parent: entry["parent"].as_u64().unwrap_or(0) as u32,
            name: entry["name"].as_str().unwrap_or_default().to_string(),
            cpu: entry["cpu"].as_f64().unwrap_or(0.0),
            private: entry["private"].as_f64().unwrap_or(0.0) as u64,
        });
    }
    let root = std::process::id();
    let mut tree = HashSet::new();
    tree.insert(root);
    loop {
        let mut added = false;
        for process in &processes {
            if tree.contains(&process.parent) && tree.insert(process.id) {
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    let mut sample = Sample::default();
    for process in &processes {
        if !tree.contains(&process.id) {
            continue;
        }
        if process.id == root {
            sample.host_cpu += process.cpu;
            sample.host_private += process.private;
            continue;
        }
        sample.webview_cpu += process.cpu;
        sample.webview_private += process.private;
        if process.name.eq_ignore_ascii_case("msedgewebview2.exe") {
            sample.webview_count += 1;
        }
    }
    Ok(sample)
}

struct Process {
    id: u32,
    parent: u32,
    name: String,
    cpu: f64,
    private: u64,
}

fn require(condition: bool, message: impl Into<String>) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn pass(message: impl std::fmt::Display) {
    println!("PASS: {message}");
}
