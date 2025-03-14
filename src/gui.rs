use rgui_events::{serde_json, Command, Event};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

use crate::{
    buffer::{self, get_buffer_create}, core::{env::ArgSlice, object::ObjectType}, editfns::insert, load, Context, Env, Rt, NIL
};

struct Handler {
    stream: TcpStream,
}

async fn handle_connection(mut stream: TcpStream, tx: mpsc::Sender<Event>, mut cmd_rx: mpsc::Receiver<Command>) -> anyhow::Result<()> {
    // Main event processing loop

    let mut buf = vec![0; 1024];
    loop {
        // Use blocking read in the thread
        match stream.read(&mut buf).await {
            Ok(n) if n > 0 => {
                if let Ok(event) = serde_json::from_slice::<Event>(&buf[..n]) {
                    if tx.send(event).await.is_err() {
                        break;
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available, sleep a bit
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            _ => break,
        }
        if let Some(cmd) = cmd_rx.recv().await {
            stream.write_all(&serde_json::to_vec(&cmd)?).await;
        }
    }
    Ok(())
}

fn bootstrap(env: &mut Rt<Env>, cx: &mut Context) -> Result<(), ()> {
    buffer::get_buffer_create(cx.add("*scratch*"), Some(NIL), cx).unwrap();
    load("bootstrap.el", cx, env)
}

pub async fn gui<'a>(env: &mut Rt<Env<'a>>, cx: &mut Context<'a>) -> anyhow::Result<()> {

    let listener = TcpListener::bind("127.0.0.1:26789").await?;
    println!("GUI server listening on 127.0.0.1:26789");

    let (stream, socket_addr) = listener.accept().await?;
    println!("Accepted connection from {socket_addr}");

    // Create channel for communication between threads
    let (tx, mut rx) = mpsc::channel(32);
    let (cmd_tx, cmd_rx) = mpsc::channel(32);

    // Spawn receiver thread
    let handle = tokio::spawn(async move {
        handle_connection(stream, tx.clone(), cmd_rx).await.unwrap();
    });
    env.txs.push(cmd_tx);

    loop {
        env.send_commands().await?;
        // process events from ui client
        if let Some(event) = rx.recv().await {
            match event {
                Event::KeyInput(key) => {
                    println!("Received key input: {:?}", key);
                    let pos = env.current_buffer.get().text.cursor().chars() as u64;
                    let ch = key.key;
                    env.stack.push(cx.add(ch));
                    // currently just call insert
                    insert(ArgSlice::new(1), env, cx)?;
                    // let content = format!("{}", ch);
                    // let cmd = Command::GridInsert { id: 0, pos, content };
                    // env.send_command(cmd).await?;
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
                    env.push_command(cmd);
                }
                Event::RequestCursorChange(cursor_change) => {
                    // let cmd = Command::CursorChange(cursor_change);
                }
                Event::Exit => {
                    break;
                }
            }
        }
    }

    handle.await?;
    Ok(())
}
