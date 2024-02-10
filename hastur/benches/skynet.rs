use async_std::task::block_on;
use criterion::Criterion;
use futures::future::{BoxFuture, FutureExt};
use hastur::*;
use std::time::Duration;

fn skynet_process(parent: Pid, num: usize, size: usize, div: usize) -> BoxFuture<'static, ()> {
    async move {
        if size == 1 {
            send(parent, num);
        } else {
            let new_size = size / div;
            let myself = myself();

            for n in 0..div {
                spawn_link(skynet_process(myself, num + n * new_size, new_size, div));
            }

            let mut sum = 0usize;

            for _n in 0..div {
                sum = receive! {
                    received: usize => {
                        sum + received
                    },
                };
            }

            send(parent, sum);
        }
    }
    .boxed()
}

async fn skynet() {
    let size = 1000000usize;
    let div = 10usize;

    let (_, _, handle) = __spawn_opt(
        async move {
            spawn_link(skynet_process(myself(), 0, size, div));

            let control = receive! {
                c: usize => {
                    c
                },
            };

            assert!(control == 499999500000);
        },
        SpawnOpt::default(),
    );

    handle.await;
}

fn main() {
    let mut criterion = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_secs(10))
        .configure_from_args();

    criterion.bench_function("skynet 1m", |bencher| bencher.iter(|| block_on(skynet())));
    criterion.final_summary();
}
