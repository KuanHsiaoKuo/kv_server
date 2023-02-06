use std::io::{Read, Write};

use crate::{CommandRequest, CommandResponse, KvError};
use bytes::{Buf, BufMut, BytesMut};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt};
use tracing::debug;

/// 长度整个占用 4 个字节
pub const LEN_LEN: usize = 4;
/// 长度占 31 bit，所以最大的 frame 是 2G
const MAX_FRAME: usize = 2 * 1024 * 1024 * 1024;
/// 如果 payload 超过了 1436 字节，就做压缩
/// 这是因为以太网的 MTU 是 1500，
/// 除去 IP 头 20 字节、TCP 头 20 字节，还剩 1460；
/// 一般 TCP 包会包含一些 Option（比如 timestamp），IP 包也可能包含，
/// 所以我们预留 20 字节；
/// 再减去 4 字节的长度，就是 1436，不用分片的最大消息长度。
/// 如果大于这个，很可能会导致分片，我们就干脆压缩一下。
const COMPRESSION_LIMIT: usize = 1436;
/// 代表压缩的 bit（整个长度 4 字节的最高位）
const COMPRESSION_BIT: usize = 1 << 31;

/// 处理 Frame 的 encode/decode
/// where clause:
/// 如果要实现这个 trait，Self 需要实现了 prost::Message
/// 大小是固定的，并且实现了 Default（prost 的需求）
/// 我们直接为 FrameCoder 提供了缺省实现
pub trait FrameCoder
where
    Self: Message + Sized + Default,
{
    /// 把一个 Message encode 成一个 frame
    /// 可以把诸如 CommandRequest 这样的消息封装成一个 frame，写入传进来的 BytesMut
    fn encode_frame(&self, buf: &mut BytesMut) -> Result<(), KvError> {
        let size = self.encoded_len();

        if size > MAX_FRAME {
            return Err(KvError::FrameError);
        }

        // 我们先写入长度，如果需要压缩，再重写压缩后的长度
        buf.put_u32(size as _);

        if size > COMPRESSION_LIMIT {
            let mut buf1 = Vec::with_capacity(size);
            self.encode(&mut buf1)?;

            // BytesMut 支持逻辑上的 split（之后还能 unsplit）
            // 所以我们先把长度这 4 字节拿走，清除
            let payload = buf.split_off(LEN_LEN);
            buf.clear();

            // 处理 gzip 压缩，具体可以参考 flate2 文档
            let mut encoder = GzEncoder::new(payload.writer(), Compression::default());
            encoder.write_all(&buf1[..])?;

            // 压缩完成后，从 gzip encoder 中把 BytesMut 再拿回来
            let payload = encoder.finish()?.into_inner();
            debug!("Encode a frame: size {}({})", size, payload.len());

            // 写入压缩后的长度
            buf.put_u32((payload.len() | COMPRESSION_BIT) as _);

            // 把 BytesMut 再合并回来
            buf.unsplit(payload);

            Ok(())
        } else {
            self.encode(buf)?;
            Ok(())
        }
    }
    /// 把一个完整的 frame decode 成一个 Message
    /// 可以把收到的一个完整的、放在 BytesMut 中的数据，
    /// 解封装成诸如 CommandRequest 这样的消息。
    fn decode_frame(buf: &mut BytesMut) -> Result<Self, KvError> {
        // 先取 4 字节，从中拿出长度和 compression bit
        let header = buf.get_u32() as usize;
        let (len, compressed) = decode_header(header);
        debug!("Got a frame: msg len {}, compressed {}", len, compressed);

        if compressed {
            // 解压缩
            let mut decoder = GzDecoder::new(&buf[..len]);
            let mut buf1 = Vec::with_capacity(len * 2);
            decoder.read_to_end(&mut buf1)?;
            buf.advance(len);

            // decode 成相应的消息
            Ok(Self::decode(&buf1[..buf1.len()])?)
        } else {
            let msg = Self::decode(&buf[..len])?;
            buf.advance(len);
            Ok(msg)
        }
    }
}

/// CommandRequest / CommandResponse 做了空实现
/// 意思是使用trait的缺省实现
impl FrameCoder for CommandRequest {}
impl FrameCoder for CommandResponse {}

fn decode_header(header: usize) -> (usize, bool) {
    let len = header & !COMPRESSION_BIT;
    let compressed = header & COMPRESSION_BIT == COMPRESSION_BIT;
    (len, compressed)
}

/// 从 stream 中读取一个完整的 frame
/// decode_frame() 函数使用的 BytesMut，是如何从 socket 里拿出来的？
/// 显然，先读 4 个字节，取出长度 N，然后再读 N 个字节。
/// 这个细节和 frame 关系很大
pub async fn read_frame<S>(stream: &mut S, buf: &mut BytesMut) -> Result<(), KvError>
where
    S: AsyncRead + Unpin + Send,
{
    let header = stream.read_u32().await? as usize;
    let (len, _compressed) = decode_header(header);
    // 如果没有这么大的内存，就分配至少一个 frame 的内存，保证它可用
    buf.reserve(LEN_LEN + len);
    buf.put_u32(header as _);
    // advance_mut 是 unsafe 的原因是，从当前位置 pos 到 pos + len，
    // 这段内存目前没有初始化。我们就是为了 reserve 这段内存，然后从 stream
    // 里读取，读取完，它就是初始化的。所以，我们这么用是安全的
    unsafe { buf.advance_mut(len) };
    stream.read_exact(&mut buf[LEN_LEN..]).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use bytes::Bytes;

    // 在开发中，也要为测试代码创建合适的生态环境，让测试简洁、可读性强。
    // 那这里，我们就创建一个简单的数据结构，使其实现 AsyncRead，
    // 这样就可以“单元”测试 read_frame() 了
    struct DummyStream {
        buf: BytesMut,
    }

    impl AsyncRead for DummyStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            // 因为只需要保证 AsyncRead 接口的正确性，所以不需要太复杂的逻辑，
            // 我们就放一个 buffer，poll_read() 需要读多大的数据，我们就给多大的数据
            // 看看 ReadBuf 需要多大的数据
            let len = buf.capacity();

            // split 出这么大的数据
            let data = self.get_mut().buf.split_to(len);

            // 拷贝给 ReadBuf
            buf.put_slice(&data);

            // 直接完工
            std::task::Poll::Ready(Ok(()))
        }
    }

    #[test]
    fn command_request_encode_decode_should_work() {
        let mut buf = BytesMut::new();

        let cmd = CommandRequest::new_hdel("t1", "k1");
        cmd.encode_frame(&mut buf).unwrap();

        // 最高位没设置
        assert!(!is_compressed(&buf));

        let cmd1 = CommandRequest::decode_frame(&mut buf).unwrap();
        assert_eq!(cmd, cmd1);
    }

    #[test]
    fn command_response_encode_decode_should_work() {
        let mut buf = BytesMut::new();

        let values: Vec<Value> = vec![1.into(), "hello".into(), b"data".into()];
        let res: CommandResponse = values.into();
        res.encode_frame(&mut buf).unwrap();

        // 最高位没设置
        assert!(!is_compressed(&buf));

        let res1 = CommandResponse::decode_frame(&mut buf).unwrap();
        assert_eq!(res, res1);
    }

    #[test]
    fn command_response_compressed_encode_decode_should_work() {
        let mut buf = BytesMut::new();

        // 有从 [u8; N] 到 Value（b"data".into()） 以及从 Bytes 到 Value 的转换，
        // 所以我们需要在 src/pb/mod.rs 里添加 From trait 的相应实现
        let value: Value = Bytes::from(vec![0u8; COMPRESSION_LIMIT + 1]).into();
        let res: CommandResponse = value.into();
        res.encode_frame(&mut buf).unwrap();

        // 最高位设置了
        assert!(is_compressed(&buf));

        let res1 = CommandResponse::decode_frame(&mut buf).unwrap();
        assert_eq!(res, res1);
    }

    #[tokio::test]
    async fn read_frame_should_work() {
        let mut buf = BytesMut::new();
        let cmd = CommandRequest::new_hdel("t1", "k1");
        cmd.encode_frame(&mut buf).unwrap();
        let mut stream = DummyStream { buf };

        let mut data = BytesMut::new();
        read_frame(&mut stream, &mut data).await.unwrap();

        let cmd1 = CommandRequest::decode_frame(&mut data).unwrap();
        assert_eq!(cmd, cmd1);
    }

    fn is_compressed(data: &[u8]) -> bool {
        if let [v] = data[..1] {
            v >> 7 == 1
        } else {
            false
        }
    }
}
