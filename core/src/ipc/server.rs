use std::sync::Arc;
use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::db::Database;
use crate::ipc::protocol::{Event, Request, Response};
use crate::perf::PerfMonitor;
use crate::runner::process::ProcessRegistry;
use crate::settings::Config;

pub struct IpcServer {
    db:       Database,
    config:   Arc<tokio::sync::RwLock<Config>>,
    runner:   Arc<crate::runner::GameRunner>,
    registry: Arc<ProcessRegistry>,
    perf:     Arc<PerfMonitor>,
    event_tx: broadcast::Sender<Event>,
}

impl IpcServer {
    pub fn new(
        db: Database,
        config: Config,
        perf: Arc<PerfMonitor>,
    ) -> (Self, broadcast::Receiver<Event>) {
        let (event_tx, event_rx) = broadcast::channel(256);
        let runner = Arc::new(crate::runner::GameRunner::new(db.clone()));
        let srv = Self {
            db:       db.clone(),
            config:   Arc::new(tokio::sync::RwLock::new(config)),
            runner,
            registry: Arc::new(ProcessRegistry::new()),
            perf,
            event_tx,
        };
        (srv, event_rx)
    }

    pub fn socket_path() -> std::path::PathBuf {
        let uid = unsafe { libc::getuid() };
        std::path::PathBuf::from(format!("/run/user/{uid}/hackeros-hm.sock"))
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let path = Self::socket_path();
        let _ = tokio::fs::remove_file(&path).await;
        let listener = UnixListener::bind(&path)?;
        info!("IPC server listening at {}", path.display());

        loop {
            let (stream, _) = listener.accept().await?;
            let srv = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(e) = srv.handle_client(stream).await {
                    warn!("IPC client error: {e}");
                }
            });
        }
    }

    async fn handle_client(self: Arc<Self>, stream: UnixStream) -> Result<()> {
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);
        let mut line   = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 { break; }

            let envelope: serde_json::Value = match serde_json::from_str(line.trim()) {
                Ok(v)  => v,
                Err(e) => { warn!("IPC parse: {e}"); continue; }
            };
            let id  = envelope["id"].clone();
            let req: Result<Request, _> = serde_json::from_value(envelope.clone());

            let response = match req {
                Ok(r)  => self.dispatch(r).await,
                Err(e) => Response::Error(format!("Invalid request: {e}")),
            };

            let reply = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": serde_json::to_value(response).unwrap_or_default(),
            });
            let mut bytes = serde_json::to_vec(&reply)?;
            bytes.push(b'\n');
            write_half.write_all(&bytes).await?;
        }
        Ok(())
    }

    async fn dispatch(&self, req: Request) -> Response {
        match req {
            Request::GetAllGames => {
                match crate::db::games::all(&self.db).await {
                    Ok(g)  => Response::Games(g),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::GetGamesBySource { source } => {
                use std::str::FromStr;
                match crate::store::StoreSource::from_str(&source) {
                    Ok(src) => match crate::db::games::by_source(&self.db, src).await {
                        Ok(g)  => Response::Games(g),
                        Err(e) => Response::Error(e.to_string()),
                    },
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::GetGame { id } => {
                match crate::db::games::by_id(&self.db, &id).await {
                    Ok(g)  => Response::Game(g),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::SetFavourite { id, favourite } => {
                match crate::db::games::set_favourite(&self.db, &id, favourite).await {
                    Ok(_)  => Response::Ok,
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::LaunchGame { id } => {
                let game = match crate::db::games::by_id(&self.db, &id).await {
                    Ok(Some(g)) => g,
                    Ok(None)    => return Response::Error(format!("Game {id} not found")),
                    Err(e)      => return Response::Error(e.to_string()),
                };
                match self.runner.launch(&game).await {
                    Ok(running) => {
                        let pid  = running.pid;
                        let _    = self.event_tx.send(Event::GameLaunched { id: id.clone(), pid });
                        let reg  = Arc::clone(&self.registry);
                        let ev   = self.event_tx.clone();
                        let id2  = id.clone();
                        reg.register(running).await;
                        tokio::spawn(async move {
                            loop {
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                if !reg.is_running(&id2).await { break; }
                            }
                            let _ = ev.send(Event::GameExited { id: id2 });
                        });
                        Response::Ok
                    }
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::KillGame { id } => {
                if self.registry.kill(&id).await {
                    Response::Ok
                } else {
                    Response::Error(format!("Game {id} not running"))
                }
            }

            Request::RunningGames => {
                Response::RunningGameIds(self.registry.running_ids().await)
            }

            Request::ImportSteam => {
                match crate::store::steam::SteamImporter::new() {
                    Some(imp) => match imp.import_all(&self.db).await {
                        Ok(n)  => { let _ = self.event_tx.send(Event::ImportProgress { source: "steam".into(), count: n }); Response::Count(n) }
                        Err(e) => Response::Error(e.to_string()),
                    },
                    None => Response::Error("Steam not found".into()),
                }
            }

            Request::ImportEpic => {
                let imp = crate::store::epic::EpicImporter::new();
                match imp.import_all(&self.db).await {
                    Ok(n)  => Response::Count(n),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::ImportGog => {
                let imp = crate::store::gog::GogImporter::new();
                match imp.import_all(&self.db).await {
                    Ok(n)  => Response::Count(n),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::ImportAmazon => {
                let imp = crate::store::amazon::AmazonImporter::new();
                match imp.import_all(&self.db).await {
                    Ok(n)  => Response::Count(n),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::ImportItchio => {
                let imp = crate::store::itchio::ItchioImporter::new();
                match imp.import_all(&self.db).await {
                    Ok(n)  => Response::Count(n),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::ImportLutris => {
                let imp = crate::store::lutris::LutrisImporter::new();
                match imp.import_all(&self.db).await {
                    Ok(n)  => Response::Count(n),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::ImportAll => {
                let db    = self.db.clone();
                let mut n = 0usize;
                if let Some(imp) = crate::store::steam::SteamImporter::new() {
                    n += imp.import_all(&db).await.unwrap_or(0);
                }
                n += crate::store::epic::EpicImporter::new().import_all(&db).await.unwrap_or(0);
                n += crate::store::gog::GogImporter::new().import_all(&db).await.unwrap_or(0);
                n += crate::store::amazon::AmazonImporter::new().import_all(&db).await.unwrap_or(0);
                n += crate::store::itchio::ItchioImporter::new().import_all(&db).await.unwrap_or(0);
                n += crate::store::lutris::LutrisImporter::new().import_all(&db).await.unwrap_or(0);
                Response::Count(n)
            }

            Request::ListProtonVersions => {
                Response::ProtonVersions(
                    crate::runner::proton::find_proton_versions()
                        .into_iter()
                        .map(|v| v.name)
                        .collect(),
                )
            }

            Request::InstallGeProton => {
                match crate::runner::proton::install_ge_proton_latest().await {
                    Ok(v)  => Response::ProtonVersions(vec![v.name]),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::GetPerfSnapshot => {
                Response::PerfSnapshot(self.perf.snapshot())
            }

            Request::ListConnections => {
                match crate::settings::network::list_connections().await {
                    Ok(c)  => Response::NetworkConnections(c),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::ListWifi => {
                match crate::settings::network::list_wifi().await {
                    Ok(n)  => Response::WifiNetworks(n),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::ConnectWifi { ssid, password } => {
                match crate::settings::network::connect_wifi(&ssid, password.as_deref()).await {
                    Ok(_)  => Response::Ok,
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::Disconnect { device } => {
                match crate::settings::network::disconnect(&device).await {
                    Ok(_)  => Response::Ok,
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::GetConfig => {
                let cfg = self.config.read().await;
                match serde_json::to_value(cfg.clone()) {
                    Ok(v)  => Response::Config(v),
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::SetConfig { config } => {
                match serde_json::from_value::<Config>(config) {
                    Ok(new_cfg) => {
                        if let Err(e) = new_cfg.save() {
                            return Response::Error(e.to_string());
                        }
                        *self.config.write().await = new_cfg;
                        Response::Ok
                    }
                    Err(e) => Response::Error(e.to_string()),
                }
            }

            Request::PowerAction { action } => {
                use crate::settings::power::{execute, PowerAction, DesktopEnvironment};
                let pa = match action.as_str() {
                    "shutdown"    => PowerAction::Shutdown,
                    "reboot"      => PowerAction::Reboot,
                    "suspend"     => PowerAction::Suspend,
                    "hibernate"   => PowerAction::Hibernate,
                    "hybrid_sleep"=> PowerAction::HybridSleep,
                    "logout"      => PowerAction::Logout,
                    "restart_hm"  => PowerAction::RestartHackerMode,
                    s if s.starts_with("switch_to:") => {
                        let de = match &s["switch_to:".len()..] {
                            "xfce"   => DesktopEnvironment::Xfce,
                            "kde"    => DesktopEnvironment::KDE,
                            "gnome"  => DesktopEnvironment::GNOME,
                            "cosmic" => DesktopEnvironment::Cosmic,
                            other    => DesktopEnvironment::Other(other.to_owned()),
                        };
                        PowerAction::SwitchToDesktop(de)
                    }
                    _ => return Response::Error(format!("Unknown power action: {action}")),
                };
                match execute(pa).await {
                    Ok(_)  => Response::Ok,
                    Err(e) => Response::Error(e.to_string()),
                }
            }
        }
    }
}
