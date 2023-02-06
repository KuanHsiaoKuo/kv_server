use anyhow::Result;
use kv2::{MemTable, ProstServerStream, Service, ServiceInner};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let addr = "127.0.0.1:9527";
    let service: Service = ServiceInner::new(MemTable::new()).into();
    let listener = TcpListener::bind(addr).await?;
    info!("Start listening on {}", addr);
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);
        let stream = ProstServerStream::new(stream, service.clone());
        // 对于服务器，我们期望可以对 accept 下来的 TcpStream
        // 提供一个 process() 方法，处理协议的细节
        // 这个 process() 方法，实际上就是对 examples/server.rs 中 tokio::spawn 里的 while loop 的封装
        tokio::spawn(async move { stream.process().await });
    }
}
