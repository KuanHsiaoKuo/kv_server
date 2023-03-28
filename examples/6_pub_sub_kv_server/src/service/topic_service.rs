use futures::{stream, Stream};
use std::{pin::Pin, sync::Arc};
use tokio_stream::wrappers::ReceiverStream;

use crate::{CommandResponse, Publish, Subscribe, Topic, Unsubscribe};

// 因为 Stream 是一个 trait，在 trait 的方法里我们无法返回
// 一个 impl Stream，所以用 trait object：Pin<Box\<dyn Stream>>。
pub type StreamingResponse = Pin<Box<dyn Stream<Item = Arc<CommandResponse>> + Send>>;

pub trait TopicService {
    /// 处理 Command，返回 Response
    fn execute(self, topic: impl Topic) -> StreamingResponse;
}

impl TopicService for Subscribe {
    fn execute(self, topic: impl Topic) -> StreamingResponse {
        let rx = topic.subscribe(self.topic);
        // 使用了 tokio-stream 的 wrapper 把一个 mpsc::Receiver
        // 转换成 ReceiverStream。这样 Subscribe 的处理就能返回一个 Stream。
        Box::pin(ReceiverStream::new(rx))
    }
}

// 对于 Unsubscribe 和 Publish，它们都返回单个值，
// 我们使用 stream::once 将其统一起来。
impl TopicService for Unsubscribe {
    fn execute(self, topic: impl Topic) -> StreamingResponse {
        let res = match topic.unsubscribe(self.topic, self.id) {
            Ok(_) => CommandResponse::ok(),
            Err(e) => e.into(),
        };
        Box::pin(stream::once(async { Arc::new(res) }))
    }
}

impl TopicService for Publish {
    fn execute(self, topic: impl Topic) -> StreamingResponse {
        topic.publish(self.topic, Arc::new(self.data.into()));
        Box::pin(stream::once(async { Arc::new(CommandResponse::ok()) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_res_error, assert_res_ok, dispatch_stream, Broadcaster, CommandRequest};
    use futures::StreamExt;
    use std::{convert::TryInto, time::Duration};
    use tokio::time;

    #[tokio::test]
    async fn dispatch_publish_should_work() {
        let topic = Arc::new(Broadcaster::default());
        let cmd = CommandRequest::new_publish("lobby", vec!["hello".into()]);
        let mut res = dispatch_stream(cmd, topic);
        let data = res.next().await.unwrap();
        assert_res_ok(&data, &[], &[]);
    }

    #[tokio::test]
    async fn dispatch_subscribe_should_work() {
        let topic = Arc::new(Broadcaster::default());
        let cmd = CommandRequest::new_subscribe("lobby");
        let mut res = dispatch_stream(cmd, topic);
        let id = get_id(&mut res).await;
        assert!(id > 0);
    }

    #[tokio::test]
    async fn dispatch_subscribe_abnormal_quit_should_be_removed_on_next_publish() {
        let topic = Arc::new(Broadcaster::default());
        let id = {
            let cmd = CommandRequest::new_subscribe("lobby");
            let mut res = dispatch_stream(cmd, topic.clone());
            let id = get_id(&mut res).await;
            drop(res);
            id as u32
        };

        // publish 时，这个 subscription 已经失效，所以会被删除
        let cmd = CommandRequest::new_publish("lobby", vec!["hello".into()]);
        dispatch_stream(cmd, topic.clone());
        time::sleep(Duration::from_millis(10)).await;

        // 如果再尝试删除，应该返回 KvError
        let result = topic.unsubscribe("lobby".into(), id);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn dispatch_unsubscribe_should_work() {
        let topic = Arc::new(Broadcaster::default());
        let cmd = CommandRequest::new_subscribe("lobby");
        let mut res = dispatch_stream(cmd, topic.clone());
        let id = get_id(&mut res).await;

        let cmd = CommandRequest::new_unsubscribe("lobby", id as _);
        let mut res = dispatch_stream(cmd, topic);
        let data = res.next().await.unwrap();

        assert_res_ok(&data, &[], &[]);
    }

    #[tokio::test]
    async fn dispatch_unsubscribe_random_id_should_error() {
        let topic = Arc::new(Broadcaster::default());

        let cmd = CommandRequest::new_unsubscribe("lobby", 9527);
        let mut res = dispatch_stream(cmd, topic);
        let data = res.next().await.unwrap();

        assert_res_error(&data, 404, "Not found: subscription 9527");
    }

    pub async fn get_id(res: &mut StreamingResponse) -> u32 {
        let id: i64 = res.next().await.unwrap().as_ref().try_into().unwrap();
        id as u32
    }
}
