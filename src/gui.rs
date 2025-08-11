use rune_gui::{build_app, create_event_loop, EventHandler, rune_events::ui_events::InputEvent};

use crate::{
    load, Context, Env, Rt
};

struct Handler {
}

impl EventHandler for Handler {

    fn handle_event(&mut self, event: InputEvent) -> anyhow::Result<()> {
        let _ = event;
        todo!()
    }
}


pub async fn gui<'a>(env: &mut Rt<Env<'a>>, cx: &mut Context<'a>) -> anyhow::Result<()> {

    let event_loop = create_event_loop()?;
    let event_handler = todo!();
    let (mut app, rx) = build_app(event_handler, &event_loop)?;

    load("gui.el", cx, env).unwrap();


    event_loop.run_app(&mut app)?;
    Ok(())
}
