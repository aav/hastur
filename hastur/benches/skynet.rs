use criterion::Criterion;
use futures::{
    future::{BoxFuture, FutureExt},
    Future,
};
use hastur::*;
use std::time::Duration;

use tokio::runtime::Builder;

use criterion::{criterion_group, criterion_main};

fn skynet_process(parent: Pid, num: usize, size: usize, div: usize) -> BoxFuture<'static, ()> {
    async move {
        if size == 1 {
            send(parent, num);
        } else {
            let new_size = size / div;
            let myself = myself();

            for n in 0..div {
                spawn(skynet_process(myself, num + n * new_size, new_size, div));
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

fn skynet_start() -> impl Future<Output = ExitReason> {
    let size = 1000000usize;
    let div = 10usize;

    let (_, _, handle) = __spawn_opt(
        async move {
            spawn(skynet_process(myself(), 0, size, div));

            let control = receive! {
                c: usize => {
                    c
                },
            };

            assert!(control == 499999500000);
        },
        SpawnOpt::default(),
    );

    handle
}

fn skynet(c: &mut Criterion) {
    c.bench_function("skynet 1m", |bencher| {
        bencher
            .to_async(Builder::new_multi_thread().build().unwrap())
            .iter(skynet_start);
    });

    c.final_summary();
}

criterion_group! {
    name = benches;
    config =
        Criterion::default()
            .sample_size(10)
            .measurement_time(Duration::from_secs(10))
            .configure_from_args();
    targets = skynet
}

criterion_main!(benches);
