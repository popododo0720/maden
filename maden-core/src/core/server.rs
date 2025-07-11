use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use hyper_util::rt::{TokioExecutor, TokioIo};
use maden_config::Config;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use rustls::ServerConfig as RustlsServerConfig;

use crate::core::http::{HttpMethod, RoutePattern};
use crate::core::service::{Handler, MadenService};
use crate::core::tls::{load_certs, load_private_key};

pub type MadenRoutes = std::sync::Arc<std::sync::Mutex<std::collections::HashMap<HttpMethod, std::collections::HashMap<RoutePattern, std::sync::Arc<crate::core::service::Handler>>>>>;

pub struct Maden {
    pub routes: MadenRoutes,
}

impl Maden {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_route(&mut self, method: HttpMethod, path: &str, query_string: Option<String>, handler: Handler) {
        let mut routes = self.routes.lock().unwrap();
        let path_map = routes.entry(method).or_default();
        let route_pattern = RoutePattern::new(path.to_string(), query_string);
        path_map.insert(route_pattern, Arc::new(handler));
    }

    pub async fn run(self, config: Config) {
        let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                maden_log::error!("Failed to bind to address {addr}: {e}");
                return;
            }
        };

        let tls_acceptor = if let Some(ssl_config) = config.ssl {
            if ssl_config.tls {
                let certs = match load_certs(std::path::Path::new(&ssl_config.cert_path)) {
                    Ok(c) => c,
                    Err(e) => {
                        maden_log::error!("Failed to load certificates: {e}");
                        return;
                    }
                };
                let key = match load_private_key(std::path::Path::new(&ssl_config.key_path)) {
                    Ok(k) => k,
                    Err(e) => {
                        maden_log::error!("Failed to load private key: {e}");
                        return;
                    }
                };

                let mut rustls_config = match RustlsServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(certs, key) {
                        Ok(c) => c,
                        Err(e) => {
                            maden_log::error!("Failed to create rustls config: {e}");
                            return;
                        }
                    };

                rustls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

                Some(TlsAcceptor::from(Arc::new(rustls_config)))
            } else {
                None
            }
        } else {
            None
        };

        maden_log::info!("Server listening on {addr}");

        loop {
            let (stream, _peer_addr) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    maden_log::error!("Failed to accept connection: {e}");
                    continue;
                }
            };
            
            if let Some(acceptor) = &tls_acceptor {
                // HTTPS connection
                let service = MadenService {
                    routes: self.routes.clone(),
                };
                let acceptor = acceptor.clone();

                tokio::spawn(async move {
                    if let Ok(tls_stream) = acceptor.accept(stream).await {
                        let io = TokioIo::new(tls_stream);
                        let hyper_service = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());

                        if let Err(err) = hyper_service.serve_connection_with_upgrades(io, service).await {
                        maden_log::error!("Error serving connection: {err:?}");
                        }
                    } else {
                        maden_log::error!("Error during TLS handshake");
                    }
                });
            } else {
                // HTTP connection
                let io = TokioIo::new(stream);
                let service = MadenService {
                    routes: self.routes.clone(),
                };

                tokio::task::spawn(async move {
                    let hyper_service = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());
                    if let Err(err) = hyper_service.serve_connection_with_upgrades(io, service).await
                    {
                        maden_log::error!("Error serving connection: {err:?}"); 
                    }
                });
            }
        }
    }
}

impl Default for Maden {
    fn default() -> Self {
        Self::new()
    }
}
