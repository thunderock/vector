use std::time::Duration;

use tokio::time::interval;
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

pub async fn io_main(proxy: EventLoopProxy<UserEvent>) {
    let mut tick_n: u64 = 0;
    let mut iv = interval(Duration::from_millis(500));
    loop {
        iv.tick().await;
        tick_n = tick_n.saturating_add(1);
        if proxy.send_event(UserEvent::Tick(tick_n)).is_err() {
            tracing::info!("event loop closed; tick task exiting");
            return;
        }
    }
}
