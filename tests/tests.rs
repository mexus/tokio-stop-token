use std::time::Duration;

use async_stream::stream;
use futures_intrusive::channel::shared::{channel, Receiver};
use tokio::{
    stream::{Stream, StreamExt},
    time::delay_for,
};

use stop_token::StopSource;

fn into_stream(receiver: Receiver<i32>) -> impl Stream<Item = i32> {
    Box::pin(stream! {
        while let Some(item) = receiver.receive().await {
            yield item;
        }
    })
}

#[test]
fn smoke() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let (sender, receiver) = channel::<i32>(10);
        let stop_source = StopSource::new();
        let task = tokio::spawn({
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
            }
        });
        sender.send(1).await.unwrap();
        sender.send(2).await.unwrap();
        sender.send(3).await.unwrap();

        delay_for(Duration::from_millis(250)).await;
        drop(stop_source);
        delay_for(Duration::from_millis(250)).await;

        sender.send(4).await.unwrap();
        sender.send(5).await.unwrap();
        sender.send(6).await.unwrap();
        assert_eq!(task.await.unwrap(), vec![1, 2, 3]);
    })
}
