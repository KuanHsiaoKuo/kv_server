use bytes::BytesMut;
use futures::{ready, FutureExt, Sink, Stream};
use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{read_frame, FrameCoder, KvError};

/// 处理 KV server prost frame 的 stream
pub struct ProstStream<S, In, Out> {
    // innner stream
    stream: S,
    // 写缓存
    wbuf: BytesMut,
    // 写入了多少字节
    written: usize,
    // 读缓存
    rbuf: BytesMut,

    // 类型占位符
    // Rust 不允许数据结构有超出需要的泛型参数。
    // 可以用 PhantomData，之前讲过它是一个零字节大小的占位符，
    // 可以让我们的数据结构携带未使用的泛型参数。
    _in: PhantomData<In>,
    _out: PhantomData<Out>,
}

impl<S, In, Out> Stream for ProstStream<S, In, Out>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    In: Unpin + Send + FrameCoder,
    Out: Unpin + Send,
{
    /// 当调用 next() 时，得到 Result<In, KvError>
    type Item = Result<In, KvError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // 上一次调用结束后 rbuf 应该为空
        assert!(self.rbuf.is_empty());

        // 从 rbuf 中分离出 rest（摆脱对 self 的引用）
        let mut rest = self.rbuf.split_off(0);

        // 使用 read_frame 来获取数据
        let fut = read_frame(&mut self.stream, &mut rest);
        // 因为 poll_xxx() 方法已经是 async/await 的底层 API 实现，
        // 所以我们在 poll_xxx() 方法中，是不能直接使用异步函数的，
        // 需要把它看作一个 future，然后调用 future 的 poll 函数。
        // 因为 future 是一个 trait，所以需要 Box 将其处理成一个
        // 在堆上的 trait object，这样就可以调用 FutureExt 的
        // poll_unpin() 方法了。
        // Box::pin 会生成 Pin。
        // ready! 宏，
        // 它会在 Pending 时直接 return Pending，
        // 而在 Ready 时，返回 Ready 的值
        ready!(Box::pin(fut).poll_unpin(cx))?;

        // 拿到一个 frame 的数据，把 buffer 合并回去
        self.rbuf.unsplit(rest);

        // 调用 decode_frame 获取解包后的数据
        Poll::Ready(Some(In::decode_frame(&mut self.rbuf)))
    }
}

/// 当调用 send() 时，会把 Out 发出去
impl<S, In, Out> Sink<Out> for ProstStream<S, In, Out>
where
    S: AsyncRead + AsyncWrite + Unpin,
    In: Unpin + Send,
    Out: Unpin + Send + FrameCoder,
{
    /// 如果发送出错，会返回 KvError
    type Error = KvError;

    // 做背压的，你可以根据负载来决定要不要返回 Poll::Ready。
    // 对于我们的网络层来说，可以先不关心背压，依靠操作系统的
    // TCP 协议栈提供背压处理即可，所以这里直接返回 Poll::Ready(Ok(()))，
    // 也就是说，上层想写数据，可以随时写
    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    // 当 poll_ready() 返回 Ready 后，Sink 就走到 start_send()。
    // 我们在 start_send() 里就把必要的数据准备好。这里把 item
    // 封包成字节流，存入 wbuf 中
    fn start_send(self: Pin<&mut Self>, item: Out) -> Result<(), Self::Error> {
        let this = self.get_mut();
        item.encode_frame(&mut this.wbuf)?;

        Ok(())
    }

    // 然后在 poll_flush() 中，我们开始写数据。
    // 这里需要记录当前写到哪里，所以需要在
    // ProstStream 里加一个字段 written，记录写入了多少字节
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();

        // 有了这个 written 字段， 就可以循环写入
        // 循环写入 stream 中
        while this.written != this.wbuf.len() {
            let n = ready!(Pin::new(&mut this.stream).poll_write(cx, &this.wbuf[this.written..]))?;
            this.written += n;
        }

        // 清除 wbuf
        this.wbuf.clear();
        this.written = 0;

        // 调用 stream 的 pull_flush 确保写入
        ready!(Pin::new(&mut this.stream).poll_flush(cx)?);
        Poll::Ready(Ok(()))
    }

    // 最后是 poll_close()，
    // 我们只需要调用 stream 的 flush 和 shutdown 方法，
    // 确保数据写完并且 stream 关闭
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // 调用 stream 的 pull_flush 确保写入
        ready!(self.as_mut().poll_flush(cx))?;

        // 调用 stream 的 pull_shutdown 确保 stream 关闭
        ready!(Pin::new(&mut self.stream).poll_shutdown(cx))?;
        Poll::Ready(Ok(()))
    }
}

// Unpin 不像 Send/Sync 会自动实现
// 一般来说，如果我们的 Stream 是 Unpin，最好实现一下
// 这会给别人在使用你的代码时带来很多方便。
// 一般来说，为异步操作而创建的数据结构，
// 如果使用了泛型参数，那么只要内部没有自引用数据，就应该实现 Unpin。
impl<S, In, Out> Unpin for ProstStream<S, In, Out> where S: Unpin {}

impl<S, In, Out> ProstStream<S, In, Out>
where
    S: AsyncRead + AsyncWrite + Send + Unpin,
{
    /// 创建一个 ProstStream
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            written: 0,
            wbuf: BytesMut::new(),
            rbuf: BytesMut::new(),
            _in: PhantomData::default(),
            _out: PhantomData::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{utils::DummyStream, CommandRequest};
    use anyhow::Result;
    use futures::prelude::*;

    #[allow(clippy::all)]
    #[tokio::test]
    async fn prost_stream_should_work() -> Result<()> {
        let buf = BytesMut::new();
        let stream = DummyStream { buf };

        // 创建 ProstStream
        let mut stream = ProstStream::<_, CommandRequest, CommandRequest>::new(stream);
        let cmd = CommandRequest::new_hdel("t1", "k1");

        // 使用 ProstStream 发送数据
        stream.send(cmd.clone()).await?;

        // 使用 ProstStream 接收数据
        if let Some(Ok(s)) = stream.next().await {
            assert_eq!(s, cmd);
        } else {
            assert!(false);
        }
        Ok(())
    }
}
