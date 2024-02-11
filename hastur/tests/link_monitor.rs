#![allow(unused_must_use, unused)]
#[cfg(test)]
mod process_tests {
    use async_metronome::{self, assert_tick, await_tick};
    use futures::channel::oneshot;
    use futures::sink::SinkExt;
    use simplelog;
    use std::time::Duration;

    use hastur::*;

    #[async_metronome::test]
    async fn spawn_nolink_panic() {
        let (pid, handle) = __spawn(async move {
            let (_, handle) = __spawn(async move {
                await_tick!(1);
                panic!();
            });

            assert_eq!(handle.await, ExitReason::Panic);
        });

        assert_eq!(handle.await, ExitReason::Normal);
    }

    #[async_metronome::test]
    async fn spawn_link_normal() {
        let (pid, _, handle) = __spawn_opt(
            async move {
                let (child, handle) = __spawn_link(async move {
                    let _ = __receive().await;
                });

                await_tick!(1);
                send(child, ());
                assert_eq!(handle.await, ExitReason::Normal);
            },
            SpawnOpt::default(),
        );

        assert_eq!(handle.await, ExitReason::Normal);
    }

    #[async_metronome::test]
    async fn spawn_link_panic() {
        let (pid, handle) = __spawn(async {
            let (pid, handle) = __spawn_link(async {
                await_tick!(1);
                panic!();
            });

            __receive().await;

            unreachable!();
        });

        assert_eq!(handle.await, ExitReason::Panic);
    }
}
