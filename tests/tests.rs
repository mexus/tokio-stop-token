use std::time::Duration;

use async_stream::stream;
use futures_intrusive::channel::shared::{channel, Receiver};
use tokio::stream::{Stream, StreamExt};

use stop_token::StopSource;

fn into_stream(receiver: Receiver<i32>) -> impl Stream<Item = i32> {
    Box::pin(stream! {
        while let Some(item) = receiver.receive().await {
            yield item;
        }
    })
}

mod task {
    use std::{future::Future, time::Duration};

    pub fn block_on<F: Future>(f: F) -> F::Output {
        tokio::runtime::Runtime::new().unwrap().block_on(f)
    }

    pub async fn spawn<F>(f: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        tokio::spawn(f).await.unwrap()
    }

    pub async fn sleep(duration: Duration) {
        tokio::time::delay_for(duration).await
    }
}

#[test]
fn smoke() {
    task::block_on(async {
        let (sender, receiver) = channel::<i32>(10);
        let stop_source = StopSource::new();
        let task = task::spawn({
            let stop_token = stop_source.stop_token();
            let receiver = receiver.clone();
            let receiver = into_stream(receiver);
            async move {
            let mut xs = Vec::new();
            let mut stream = stop_token.stop_stream(receiver);
            while let Some(x) = stream.next().await {
                xs.push(x)
            }
            xs
        }});
        sender.send(1).await.unwrap();
        sender.send(2).await.unwrap();
        sender.send(3).await.unwrap();

        task::sleep(Duration::from_millis(250)).await;
        drop(stop_source);
        task::sleep(Duration::from_millis(250)).await;

        sender.send(4).await.unwrap();
        sender.send(5).await.unwrap();
        sender.send(6).await.unwrap();
        assert_eq!(task.await, vec![1, 2, 3]);
    })
}
