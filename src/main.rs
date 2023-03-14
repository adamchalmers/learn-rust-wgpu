mod draw;
mod texture;

fn main() {
    // Reminder, never use block_on inside an async fn if you're running in WASM.
    // Why? Futures have to be run on the browser's executor. So you can't BYO.
    pollster::block_on(draw::run());
}
