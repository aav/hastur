use hastur::*;

#[tokio::main]
async fn main() {
    std::panic::set_hook(Box::new(|_| {}));

    let (_, handle) = __spawn(async {
        let (_, handle) = __spawn_link(async {
            panic!();
        });

        handle.await;
    });

    handle.await;
}
