use std::{
    error::Error,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisRequest {
    id: String,
    work: Work,
    engine: Engine,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    let analysis_request = ureq::get("http://localhost:3000/api/external-engine/work")
        .call()?
        .into_json::<AnalysisRequest>()?;

    println!("{analysis_request:#?}");

    // Step 2) Start a POST request stream to /api/external-engine/work/{id}
    // http://localhost:3000/api/external-engine/work/{}
    // todo

    // Step 3) Send the FEN to the engine
    let mut engine = Command::new("./stockfish")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let engine_stdin = engine.stdin.as_mut().ok_or("Failed to get stdin")?;

    // Set UCI options
    writeln!(
        engine_stdin,
        "setoption name Threads value {}",
        analysis_request.work.threads
    )?;
    writeln!(
        engine_stdin,
        "setoption name Hash value {}",
        analysis_request.work.hash
    )?;
    writeln!(
        engine_stdin,
        "setoption name MultiPV value {}",
        analysis_request.work.multi_pv
    )?;
    writeln!(
        engine_stdin,
        "setoption name Variant value {}",
        analysis_request.work.variant
    )?;
    writeln!(
        engine_stdin,
        "position fen {} moves {}",
        analysis_request.work.initial_fen,
        analysis_request.work.moves.join(" ")
    )?;

    if analysis_request.work.infinite {
        writeln!(engine_stdin, "go infinite")?;
    } else {
        writeln!(
            engine_stdin,
            "go depth {}\n",
            analysis_request.engine.default_depth
        )?;
    }

    engine_stdin.flush()?;

    let engine_stdout = engine.stdout.as_mut().ok_or("Failed to get stdout")?;

    for line in BufReader::new(engine_stdout).lines() {
        let line = line?;
        println!("Engine: {}", line.trim());
        if line.contains("info") {
            // Step 4) Send the "info" line to the server
            // todo
        }
        if line.contains("bestmove") {
            println!("Found bestmove: {}", line);
            break;
        }
    }

    Ok(())
}
