#[cfg(test)]
mod process_tests {
    use futures::channel::oneshot;
    use hastur::*;

    // self returns pid and they are consistent
    #[async_metronome::test]
    async fn spawn_normal() {
        let (s, r) = oneshot::channel::<Pid>();

        let (pid, _, handle) = __spawn_opt(
            async {
                let pid = myself();
                let pid1 = myself();

                assert_eq!(pid, pid1);

                s.send(pid).unwrap();
            },
            SpawnOpt::default(),
        );

        // no crash
        assert_eq!(handle.await, ExitReason::Normal);

        // pid is the same as inside process
        assert_eq!(pid, r.await.unwrap());

        // new pid on for new proc
        let (pid1, _, handle) = __spawn_opt(async {}, SpawnOpt::default());
        handle.await;

        assert_ne!(pid, pid1);
    }

    // self returns pid and they are consistent
    #[test]
    #[should_panic]
    fn no_process_no_pid_() {
        // should fail when called outside of a process
        let _ = myself();
    }

    // normal termination -> ExitReason::Normal
    #[async_metronome::test]
    async fn exit_normal() {
        let (_, _, handle) = __spawn_opt(
            async {
                //
            },
            SpawnOpt::default(),
        );
        assert_eq!(handle.await, ExitReason::Normal);
    }

    // panic termination -> ExitReason::Panic
    #[async_metronome::test]
    async fn exit_panic() {
        let (_, _, handle) = __spawn_opt(
            async {
                panic!();
            },
            SpawnOpt::default(),
        );

        assert_eq!(handle.await, ExitReason::Panic);
    }

    #[async_metronome::test]
    async fn self_exit() {
        let (_, _, handle) = __spawn_opt(
            async move {
                exit(ExitReason::Normal).await;
                unreachable!();
            },
            SpawnOpt::default(),
        );

        assert_eq!(handle.await, ExitReason::Normal);
    }
}
