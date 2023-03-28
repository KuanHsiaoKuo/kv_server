use anyhow::Result;
use kv2::{CommandRequest, ProstClientStream};
use tokio::net::TcpStream;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:9527";
    // 连接服务器
    let stream = TcpStream::connect(addr).await?;

    let mut client = ProstClientStream::new(stream);

    // 生成一个 HSET 命令
    let cmd = CommandRequest::new_hset("table1", "hello", "world".to_string().into());

    // 发送 HSET 命令
    // 对客户端，我们也希望可以直接 execute() 一个命令，就能得到结果
    let data = client.execute(cmd).await?;
    info!("Got response {:?}", data);

    Ok(())
}
