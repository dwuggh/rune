use rgui_events::{serde_json, Command, Event};
use std::{
    io::{Read, Write},
    net::TcpStream,
};

use crate::{
    buffer::get_buffer_create,
    core::{env::ArgSlice, object::ObjectType},
    editfns::insert,
    Context, Env, Rt,
};

pub fn gui(env: &mut Rt<Env>, cx: &mut Context) -> anyhow::Result<()> {
    let listener = std::net::TcpListener::bind("127.0.0.1:26789")?;
    println!("GUI server listening on 127.0.0.1:26789");
    let (mut stream, socket_addr) = listener.accept()?;
    println!("Accepted connection from {socket_addr}");
    let mut buf = vec![0; 1024];
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            continue;
        }
        if let Ok(event) = serde_json::from_slice::<Event>(&buf[..n]) {
            match event {
                Event::KeyInput(key) => {
                    println!("Received key input: {:?}", key);
                    let pos = env.current_buffer.get().text.cursor().chars() as u64;
                    let ch = key.key;
                    env.stack.push(cx.add(ch));
                    insert(ArgSlice::new(1), env, cx)?;
                    let content = format!("{}", ch);
                    let cmd = Command::GridInsert { id: 0, pos, content };
                    stream.write(&serde_json::to_vec(&cmd)?)?;
                }
                Event::RequestBufferContent { buffer, start, len } => {
                    let buf = env.current_buffer.get();

                    let start = start as usize;
                    let text = &buf.text;
                    let buf_len = text.len_chars();
                    if start > buf_len {
                        continue;
                    }
                    let end =
                        if start + len as usize > buf_len { buf_len } else { start + len as usize };
                    let (a, b) = text.slice(start..end);
                    let content = format!("{}{}", a, b);
                    let cmd = Command::GridInsert { id: 0, pos: start as u64, content };
                    stream.write(&serde_json::to_vec(&cmd)?)?;
                }
                Event::RequestCursorChange(cursor_change) => {
                    let command = Command::CursorChange(cursor_change);
                    let command = serde_json::to_vec(&command).unwrap();
                    stream.write(&command).unwrap();
                }
            }
        }
    }
}
