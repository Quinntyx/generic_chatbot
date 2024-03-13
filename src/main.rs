use kalosm::language::*;
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::io::{stdin, Write};

use ron::{de, ser};

fn print(s: &str) -> io::Result<()> {
    print!("{}", s);
    std::io::stdout().flush()
}

fn build(message: (bool, String)) -> String {
    if message.0 {
        format!("<|user|>{}</s>", message.1)
    } else {
        format!("<|assistant|>{}</s>", message.1)
    }
}

fn write_next_message_content(msg: &mut String, msg_queue: &mut VecDeque<String>) {
    msg_queue
        .pop_front()
        .map(|a| {
            println!("{a}");
            *msg = a
        })
        .unwrap_or_else(|| {
            stdin().read_line(msg).unwrap();
        });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Initializing Zephyr 7B Alpha (Llama)... ");

    let llm = LlamaBuilder::default()
        .with_source(LlamaSource::zephyr_7b_alpha())
        .build().await?;
    println!("done!");

    let mut msgq = fs::read_to_string("msgq.ron")
        .map(|m| de::from_str(&m).unwrap())
        .unwrap_or_else(|_| VecDeque::new());

    let base_prompt = String::from(
        "<|system|>Your name is Fiona. You are extremely smart, helpful, and take pride in having knowledge and solving difficult problems. You are also uptight \
        and curt with responses.</s>"
    );

    let mut old_messages: Vec<(bool, String)> = Vec::new();

    let mut messages: Vec<(bool, String)> = Vec::new();

    let mut current = 0;

    loop {
        let mut message = String::new();

        print(&format!("[{current}] User: "))?;

        write_next_message_content(&mut message, &mut msgq);

        message.remove(message.len() - 1); // always a newline

        if message.contains("EXIT") {
            break;
        }

        if message.contains("RESET") {
            println!("Resetting!");
            old_messages.append(&mut messages);

            fs::write(
                "conversation.ron",
                ser::to_string_pretty(&old_messages, Default::default()).unwrap(),
            )?;
            continue;
        }

        messages.push((true, message.clone()));

        current += 1;

        let prompt = format!(
            "{}\n{}\n<|assistant|>",
            base_prompt,
            &messages
                .iter()
                .map(|m| build(m.clone()))
                .collect::<Vec<String>>()
                .join("\n")
        );

        let stream = llm
            .stream_text(&prompt)
            .with_stop_on(Some(String::from("</s>")))
            .await
            .unwrap();

        let mut response = String::new();

        let mut sentences = stream.words();
        while let Some(text) = sentences.next().await {
            // print(&text);
            print(".")?;
            response.push_str(&text)
        }

        println!();

        messages.push((false, response.trim_end_matches("\n").to_string()));

        print!("[{current}] Fiona: ");
        std::io::stdout().flush()?;
        println!("{}", &messages.last().unwrap().1);

        current += 1;
    }

    old_messages.append(&mut messages);

    println!("Exiting!");

    fs::write(
        "conversation.ron",
        ser::to_string_pretty(&old_messages, Default::default()).unwrap(),
    )?;

    Ok(())
}
