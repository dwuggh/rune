use std::sync::Arc;

use anyhow::anyhow;
use rune_gui::{app::{app_logic, AppData}, xilem::{self, tokio, winit::platform::wayland::EventLoopBuilderExtWayland, EventLoop}, EventHandler, EventReceiver};

use crate::{core::{env::Env, gc::{Context, Rt}, object::ObjectType}, frame::selected_frame, load};

struct ChannelEventHandler {
    tx: tokio::sync::mpsc::Sender<rune_gui::rune_events::InputEvent>
}

impl ChannelEventHandler {
    fn new() -> (ChannelEventHandler, tokio::sync::mpsc::Receiver<rune_gui::rune_events::InputEvent>) {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let handler = ChannelEventHandler {
            tx,
        };
        (handler, rx)
    }
}

impl EventHandler for ChannelEventHandler {
    fn handle_event(&self, event: rune_gui::rune_events::InputEvent) -> anyhow::Result<()> {
        self.tx.blocking_send(event)?;
        Ok(())
    }
}

pub fn gui(env: &mut Rt<Env>, cx: &mut Context) -> anyhow::Result<()> {
    let (handler, mut rx) = ChannelEventHandler::new();
    load("gui.el", cx, env).unwrap();
    let frame = selected_frame(env);
    let ObjectType::Frame(frame) = frame.untag() else {
        anyhow::bail!("err");
    };
    let vm = frame.viewmodel(env, cx);
    let handler = Arc::new(handler);
    let state = AppData::new(vm, handler);
    let handle = std::thread::spawn(move || {
        println!("open other thread");
        let app = xilem::Xilem::new(state, app_logic);
        let mut event_loop_builder = EventLoop::with_user_event();
        event_loop_builder.with_any_thread(true);
        app.run_in(event_loop_builder).unwrap();
    });

    while let Some(event) = rx.blocking_recv() {
        println!("{event:?}");
    }
    Ok(())
}