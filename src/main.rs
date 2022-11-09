#![allow(dead_code)]

use std::{
    error::Error,
    io::{BufRead, BufReader, Read, Write},
    vec,
};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisRequest {
    id: String,
    work: Work,
    engine: Engine,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Work {
    session_id: String,
    threads: u32,
    hash: u32,
    infinite: bool,
    multi_pv: u32,
    variant: String,
    initial_fen: String,
    moves: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Engine {
    id: String,
    name: String,
    client_secret: String,
    user_id: String,
    max_threads: u32,
    max_hash: u32,
    default_depth: u32,
    variants: Vec<String>,
    provider_data: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Step 1) Long poll for analysis requests
    // When a move is made on the Analysis board, it will be returned from this endpoint
    let analysis_request =
        reqwest::blocking::get("http://localhost:3000/api/external-engine/work")?
            .json::<AnalysisRequest>()?;

    println!("{:#?}", analysis_request);

    // Step 2) Start a POST request stream to /api/external-engine/work/{id}
    // let analysis_answer = reqwest::Client::new()
    //     .post(format!(
    //         "http://localhost:3000/api/external-engine/work/{}",
    //         analysis_request.id
    //     ))

    // Step 3) Send the FEN to the engine
    let fen = analysis_request.work.initial_fen;
    println!("FEN: {}", fen);
    let mut engine = std::process::Command::new("./stockfish")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    println!("Engine started");

    let engine_stdin = engine.stdin.as_mut().ok_or("Failed to get stdin")?;

    let _ = engine_stdin.write_all(format!("position fen {}\n", fen).as_bytes());
    let _ = engine_stdin.write_all(b"go depth 20\n");

    engine_stdin.flush()?;

    let engine_stdout = engine.stdout.as_mut().ok_or("Failed to get stdout")?;

    let mut reader = BufReader::new(engine_stdout);
    loop {
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        println!("Engine: {}", line.trim());
        if line.contains("bestmove") {
            println!("Found bestmove: {}", line);
            break;
        }
    }

    Ok(())
}
