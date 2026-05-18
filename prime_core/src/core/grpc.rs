use tokio::sync::RwLock;

pub struct GrpcServer {
    running: RwLock<bool>,
    addr: RwLock<String>,
}

impl Default for GrpcServer {
    fn default() -> Self {
        Self::new()
    }
}

impl GrpcServer {
    pub fn new() -> Self {
        Self {
            running: RwLock::new(false),
            addr: RwLock::new("127.0.0.1:50051".to_string()),
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let addr = self.addr.read().await.clone();
        let _socket_addr: std::net::SocketAddr = addr.parse()?;

        *self.running.write().await = true;
        tracing::info!("gRPC server starting on {}", addr);

        // In production, register actual services here
        // Server::builder()
        //     .add_service(prime_proto::prime_server::PrimeServer::new(PrimeService))
        //     .serve(socket_addr)
        //     .await?;

        Ok(())
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;
        tracing::info!("gRPC server stopped");
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}
