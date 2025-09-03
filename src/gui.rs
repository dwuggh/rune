use std::sync::Arc;

use anyhow::anyhow;
use rune_gui::{
    EventHandler, EventReceiver,
    app::{AppData, app_logic, init_rx},
    rune_events::TextEvent,
    xilem::{self, EventLoop, tokio, winit::platform::wayland::EventLoopBuilderExtWayland},
};

use crate::{
    cmds::self_insert_command,
    core::{
        env::Env,
        gc::{Context, Rt},
        object::ObjectType,
    },
    frame::selected_frame,
    load,
};

struct ChannelEventHandler {
    tx: tokio::sync::mpsc::Sender<rune_gui::rune_events::InputEvent>,
}

impl ChannelEventHandler {
    fn new() -> (
        ChannelEventHandler,
        tokio::sync::mpsc::Receiver<rune_gui::rune_events::InputEvent>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let handler = ChannelEventHandler { tx };
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
    std::thread::spawn(move || {
        println!("open other thread");
        let app = xilem::Xilem::new(state, app_logic);
        let mut event_loop_builder = EventLoop::with_user_event();
        event_loop_builder.with_any_thread(true);
        app.run_in(event_loop_builder).unwrap();
    });

    let tx = init_rx();

    while let Some(event) = rx.blocking_recv() {
        match event {
            rune_gui::rune_events::InputEvent::TextEvent(text_event) => {
                if let TextEvent::Keyboard(event) = text_event {
                    if event.state.is_up() {
                        match event.key {
                            rune_gui::rune_events::keyboard_types::Key::Character(c) => {
                                let ch = c.chars().next().unwrap();
                                self_insert_command(1, ch, env, cx);
                                let frame = selected_frame(env);
                                let ObjectType::Frame(frame) = frame.untag() else {
                                    anyhow::bail!("err");
                                };
                                let viewmodel = frame.viewmodel(env, cx);
                                tx.blocking_send(viewmodel).unwrap();
                            }
                            rune_gui::rune_events::keyboard_types::Key::Named(named_key) => {}
                        }
                    }
                }
            }
            rune_gui::rune_events::InputEvent::PointerEvent => {}
        }
    }
    Ok(())
}
