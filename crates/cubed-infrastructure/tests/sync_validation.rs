use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cubed_application::ports::{
    ConsoleManager, ModpackRepository, PortManager, ProcessManager, ResourceMonitor,
    ServerRepository,
};
use cubed_application::use_cases::{CreateServer, CreateServerInput};
use cubed_domain::entities::{ServerSoftware, ServerStatus};
use cubed_infrastructure::{
    FileBackupManager, FileModManager, InMemoryBackupRepo, InMemoryModRepo, InMemoryModpackRepo,
    JsonServerRepository, LocalFileSystem, MinecraftConsoleManager, MinecraftProcessManager,
    ModpackInstaller, SysInfoResourceMonitor, TcpPortManager,
};
use tempfile::tempdir;
use tokio::sync::mpsc;

async fn wait_until<F, Fut>(timeout: Duration, mut check: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if check().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

fn write_zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::<()>::default();
    for (name, bytes) in entries {
        zip.start_file(name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

#[tokio::test]
async fn sync_validation_covers_server_process_console_mods_modpacks_backups_and_metrics() {
    let tmp = tempdir().unwrap();
    let servers_dir = tmp.path().join("servers");
    let backups_dir = tmp.path().join("backups");
    let repo_path = tmp.path().join("servers.json");

    let repo = Arc::new(JsonServerRepository::new(repo_path.clone()));
    let fs = Arc::new(LocalFileSystem::new(
        tmp.path().to_string_lossy().to_string(),
    ));
    let port = TcpPortManager::new().find_free_from(25_565).await.unwrap();

    let server = CreateServer::new(repo.clone(), fs)
        .execute(CreateServerInput {
            name: "sync-test".into(),
            version: "1.21".into(),
            software: ServerSoftware::Paper,
            port,
            java_path: "/usr/bin/java".into(),
            servers_dir: servers_dir.to_string_lossy().to_string(),
        })
        .await
        .unwrap();

    let server_dir = servers_dir.join("sync-test");
    assert!(server_dir.join("mods").is_dir());
    assert_eq!(server.port().value(), port);

    let reloaded_repo = JsonServerRepository::new(repo_path);
    let persisted = reloaded_repo
        .find_by_id(server.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(persisted.name().as_str(), "sync-test");
    assert_eq!(persisted.port().value(), port);

    let run_script = server_dir.join("run.sh");
    std::fs::write(
        &run_script,
        format!(
            r#"#!/bin/sh
exec python3 -u -c '
import socket, sys, time
port = {port}
sock = socket.socket()
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.bind(("127.0.0.1", port))
sock.listen(1)
print("stdout-ready", flush=True)
print("Done (0.001s)! For help, type \"help\"", flush=True)
print("stderr-ready", file=sys.stderr, flush=True)
while True:
    time.sleep(0.1)
'
"#
        ),
    )
    .unwrap();

    let process_mgr = MinecraftProcessManager::new();
    let console = MinecraftConsoleManager::new();
    let (pid, stdin, stdout, stderr) = process_mgr
        .spawn_script_with_io(
            server.id(),
            run_script.to_str().unwrap(),
            server_dir.to_str().unwrap(),
        )
        .await
        .unwrap();
    assert!(pid > 0);
    assert!(process_mgr.is_alive(server.id()).await.unwrap());
    console.register_stdin(server.id(), stdin).await;

    let (tx, mut rx) = mpsc::unbounded_channel();
    console
        .attach(
            server.id(),
            Box::new(move |line| {
                let _ = tx.send(line);
            }),
        )
        .await
        .unwrap();
    console.spawn_readers(server.id(), stdout, stderr).await;

    let mut saw_stdout = false;
    let mut saw_stderr = false;
    let mut saw_done = false;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && !(saw_stdout && saw_stderr && saw_done) {
        if let Ok(Some(line)) = tokio::time::timeout(Duration::from_millis(250), rx.recv()).await {
            saw_stdout |= line.is_stdout && line.text.contains("stdout-ready");
            saw_stderr |= !line.is_stdout && line.text.contains("stderr-ready");
            saw_done |=
                line.is_stdout && line.text.contains("Done") && line.text.contains("For help");
        }
    }
    assert!(saw_stdout);
    assert!(saw_stderr);
    assert!(saw_done);

    let port_open = wait_until(Duration::from_secs(3), || async {
        TcpStream::connect(("127.0.0.1", port)).is_ok()
    })
    .await;
    assert!(port_open);

    let mut running_server = persisted.clone();
    running_server.start().unwrap();
    running_server.mark_running().unwrap();
    repo.save(&running_server).await.unwrap();
    assert!(matches!(
        repo.find_by_id(server.id())
            .await
            .unwrap()
            .unwrap()
            .status(),
        ServerStatus::Running
    ));

    let stats = SysInfoResourceMonitor::new()
        .server_stats(server.id(), pid)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stats.server_id, server.id());
    assert!(stats.uptime_secs <= 10);
    let system_stats = SysInfoResourceMonitor::new().system_stats().await.unwrap();
    assert!(system_stats.ram_total_bytes > 0);
    assert!(system_stats.disk_total_bytes > 0);

    let mod_source = tmp.path().join("example.jar");
    std::fs::write(&mod_source, b"PK\x03\x04test").unwrap();
    let mod_repo = InMemoryModRepo::new();
    let mod_mgr = FileModManager::new(repo.clone(), mod_repo);
    let installed_mod = mod_mgr
        .install_mod(
            server.id(),
            mod_source.to_str().unwrap(),
            server_dir.join("mods").to_str().unwrap(),
        )
        .await
        .unwrap();
    assert!(std::path::Path::new(installed_mod.path()).is_file());

    let modpack_zip = tmp.path().join("pack.zip");
    write_zip(
        &modpack_zip,
        &[
            ("mods/packmod.jar", b"PK\x03\x04pack"),
            ("config/example.toml", b"enabled=true"),
        ],
    );
    let modpack_repo = InMemoryModpackRepo::new();
    let modpack_installer = ModpackInstaller::new(repo.clone(), modpack_repo.clone());
    let (modpack, summary) = modpack_installer
        .install(
            server.id(),
            modpack_zip.to_str().unwrap(),
            server_dir.to_str().unwrap(),
            |_| {},
        )
        .await
        .unwrap();
    assert_eq!(summary.total_files, 2);
    assert_eq!(summary.downloaded, 2);
    assert!(server_dir.join("mods/packmod.jar").is_file());
    assert!(server_dir.join("config/example.toml").is_file());
    assert!(modpack_repo
        .find_by_id(modpack.id())
        .await
        .unwrap()
        .is_some());

    std::fs::write(server_dir.join("server.properties"), b"motd=sync-test").unwrap();
    let backup_repo = InMemoryBackupRepo::new();
    let backup_mgr =
        FileBackupManager::new(backups_dir.to_string_lossy(), repo.clone(), backup_repo);
    let backup = backup_mgr
        .backup_server(server.id(), "sync-test", server_dir.to_str().unwrap())
        .await
        .unwrap();
    assert!(std::path::Path::new(backup.path()).is_file());

    let restore_dir = tmp.path().join("restore");
    backup_mgr
        .restore_backup(backup.id(), restore_dir.to_str().unwrap())
        .await
        .unwrap();
    assert!(
        std::fs::read_to_string(restore_dir.join("sync-test/server.properties"))
            .unwrap()
            .contains("motd=sync-test")
    );

    process_mgr.kill(server.id()).await.unwrap();
    assert!(!process_mgr.is_alive(server.id()).await.unwrap());
}
