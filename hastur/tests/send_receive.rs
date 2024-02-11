#![allow(unused_must_use, unused)]
#[cfg(test)]
mod process_tests {
    use async_metronome::{self, assert_tick, await_tick};
    use futures::channel::oneshot;
    use futures::sink::SinkExt;
    use std::time::Duration;

    use hastur::*;

    #[async_metronome::test]
    async fn basic_send() {
        let (pid, _, handle) = __spawn_opt(
            async move {
                let message = __receive().await;
                assert_tick!(1);
                assert_eq!(message, ());
            },
            SpawnOpt::default(),
        );

        await_tick!(1);
        send(pid, ());

        assert_eq!(handle.await, ExitReason::Normal);
    }

    #[async_metronome::test(debug = false)]
    async fn basic_send_order() {
        let (pid, _, handle) = __spawn_opt(
            async move {
                await_tick!(1);

                for n in 1..10 {
                    let message = __receive().await;
                    assert_eq!(message, n);
                }
            },
            SpawnOpt::default(),
        );

        for n in 1..10 {
            send(pid, n);
        }

        assert_eq!(handle.await, ExitReason::Normal);
    }

    #[async_metronome::test]
    async fn basic_send_self() {
        let (pid, _, handle) = __spawn_opt(
            async move {
                send(myself(), ());
                let message = __receive().await;
                assert_eq!(message, ());
            },
            SpawnOpt::default(),
        );

        assert_eq!(handle.await, ExitReason::Normal);
    }
}
