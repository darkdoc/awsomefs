use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_log::LogTracer::init().expect("Failed to set up LogTracer");

    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
    
    let addr = "127.0.0.1:50051".parse()?;
    tracing::info!("Metadata service listening on {}", addr);

    Server::builder()
        .add_service(metadata_service::server::build_metadata_server())
        .serve(addr)
        .await?;

    Ok(())
}