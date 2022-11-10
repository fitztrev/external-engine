use std::{
    error::Error,
    io::{BufRead, BufReader, Write},
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
    let analysis_request =
        reqwest::blocking::get("http://localhost:3000/api/external-engine/work")?
            .json::<AnalysisRequest>()?;

    println!("{:#?}", analysis_request);

    // Step 2) Start a POST request stream to /api/external-engine/work/{id}
    // http://localhost:3000/api/external-engine/work/{}
    // todo

    // Step 3) Send the FEN to the engine
    let mut engine = std::process::Command::new("./stockfish")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let engine_stdin = engine.stdin.as_mut().ok_or("Failed to get stdin")?;

    // Set UCI options
    engine_stdin.write_all(
        format!(
            "setoption name Threads value {}\n",
            analysis_request.work.threads
        )
        .as_bytes(),
    )?;
    engine_stdin.write_all(
        format!("setoption name Hash value {}\n", analysis_request.work.hash).as_bytes(),
    )?;
    engine_stdin.write_all(
        format!(
            "setoption name MultiPV value {}\n",
            analysis_request.work.multi_pv
        )
        .as_bytes(),
    )?;
    engine_stdin.write_all(
        format!(
            "setoption name Variant value {}\n",
            analysis_request.work.variant
        )
        .as_bytes(),
    )?;

    let _ = engine_stdin.write_all(
        format!(
            "position fen {} moves {}\n",
            analysis_request.work.initial_fen,
            analysis_request.work.moves.join(" ")
        )
        .as_bytes(),
    );

    if analysis_request.work.infinite {
        let _ = engine_stdin.write_all(b"go infinite\n");
    } else {
        let _ = engine_stdin
            .write_all(format!("go depth {}\n", analysis_request.engine.default_depth).as_bytes());
    }

    engine_stdin.flush()?;

    let engine_stdout = engine.stdout.as_mut().ok_or("Failed to get stdout")?;

    let mut reader = BufReader::new(engine_stdout);
    loop {
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
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
