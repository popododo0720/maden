use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};

use hyper_util::rt::{TokioExecutor, TokioIo};
use maden_config::Config;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use rustls::ServerConfig as RustlsServerConfig;

use crate::core::http::HttpMethod;
use crate::core::service::{Handler, MadenService};
use crate::core::tls::{load_certs, load_private_key};

pub type MadenRoutes = Arc<HashMap<HttpMethod, matchit::Router<Arc<Handler>>>>;

pub struct Maden {
    pub routes: HashMap<HttpMethod, matchit::Router<Arc<Handler>>>,
}

impl Maden {
    pub fn new() -> Self {
        let mut routes = HashMap::new();
        routes.insert(HttpMethod::Get, matchit::Router::new());
        routes.insert(HttpMethod::Post, matchit::Router::new());
        routes.insert(HttpMethod::Put, matchit::Router::new());
        routes.insert(HttpMethod::Delete, matchit::Router::new());
        routes.insert(HttpMethod::Patch, matchit::Router::new());
        routes.insert(HttpMethod::Options, matchit::Router::new());
        routes.insert(HttpMethod::Head, matchit::Router::new());

        Self {
            routes
        }
    }

    pub fn add_route(&mut self, method: HttpMethod, path: &str, _query_string: Option<String>, handler: Handler) {
        if let Some(router) = self.routes.get_mut(&method) {
            if let Err(e) = router.insert(path, Arc::new(handler)) {
                maden_log::error!("Failed to insert route {path}: {e}");
            }
        }
    }

    pub async fn run(self, config: Config) {
        let routes = Arc::new(self.routes);
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
                    routes: routes.clone(),
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
                    routes: routes.clone(),
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
