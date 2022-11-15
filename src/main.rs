use std::{
    error::Error,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

use rand::Rng;
use reqwest::{
    blocking::{Body, Client, ClientBuilder},
    header::{self, HeaderMap},
};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisRequest {
    id: String,
    work: Work,
    engine: Engine,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
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
    provider_data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Registration {
    name: String,
    max_threads: u32,
    max_hash: u32,
    default_depth: u32,
    variants: Vec<String>,
    provider_secret: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkRequest {
    provider_secret: String,
}

const ENGINE_NAME: &str = "Rust Test Engine";
const ENGINE_BINARY: &str = "./stockfish";
const LICHESS_HOST: &str = "https://lichess.org";
const ENGINE_HOST: &str = "https://engine.lichess.ovh";

fn register_engine(client: &Client) -> Result<String, Box<dyn Error>> {
    let engines = client
        .get(format!("{}/api/external-engine", LICHESS_HOST))
        .send()?
        .json::<Vec<Engine>>()?;

    let secret = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(128)
        .map(char::from)
        .collect::<String>();
    let registration = Registration {
        name: ENGINE_NAME.to_string(),
        max_threads: std::thread::available_parallelism()
            .map(|v| v.get() as _)
            .unwrap_or(1),
        max_hash: 512,
        default_depth: 25,
        variants: vec!["chess".to_string()],
        provider_secret: secret.clone(),
    };

    for engine in engines {
        if engine.name == ENGINE_NAME {
            println!("Updating engine {}", engine.id);
            client
                .put(format!(
                    "{}/api/external-engine/{}",
                    LICHESS_HOST, engine.id
                ))
                .json(&registration)
                .send()?;
            return Ok(secret);
        }
    }

    println!("Registering new engine");
    client
        .post(format!("{}/api/external-engine", LICHESS_HOST))
        .json(&registration)
        .send()?;

    Ok(secret)
}

fn main() -> Result<(), Box<dyn Error>> {
    let auth_header = if let Some(token) = std::env::args().nth(1) {
        format!("Bearer {token}")
    } else {
        println!("Pass token from {}/account/oauth/token/create?scopes[]=engine:read&scopes[]=engine:write as argument", LICHESS_HOST);
        return Ok(());
    };
    let mut default_headers = HeaderMap::new();
    default_headers.insert(header::AUTHORIZATION, auth_header.try_into()?);
    let client = ClientBuilder::new()
        .default_headers(default_headers)
        .build()?;

    let provider_secret = register_engine(&client)?;

    loop {
        // Step 1) Long poll for analysis requests
        // When a move is made on the Analysis board, it will be returned from this endpoint
        let response = client
            .post(format!("{}/api/external-engine/work", ENGINE_HOST))
            .json(&WorkRequest {
                provider_secret: provider_secret.clone(),
            })
            .send()?;

        if response.status() != 200 {
            continue;
        }

        let analysis_request = response.json::<AnalysisRequest>()?;
        println!("{analysis_request:#?}");

        // Step 2) Send the FEN to the engine
        let mut engine = Command::new(ENGINE_BINARY)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let engine_stdin = engine.stdin.as_mut().ok_or("Failed to get stdin")?;

        // Set UCI options
        writeln!(engine_stdin, "setoption name UCI_AnalyseMode value true")?;
        writeln!(engine_stdin, "setoption name UCI_Chess960 value true")?;
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

        let (tx, rx) = std::sync::mpsc::channel();
        let client = client.clone();

        std::thread::spawn(move || {
            // Step 3) Start a POST request stream to /api/external-engine/work/{id}
            client
                .post(format!(
                    "{}/api/external-engine/work/{}",
                    ENGINE_HOST, analysis_request.id
                ))
                .body(Body::new(iter_read::IterRead::new(rx.into_iter().fuse())))
                .send()
        });

        for line in BufReader::new(engine_stdout).lines() {
            let mut line = line?;
            println!("Engine: {}", line);
            if line.starts_with("info") {
                line.push('\n');
                if tx.send(line).is_err() {
                    // sending thread stopped, meaning Lichess doesn't want any more analysis. wait for next request
                    break;
                }
            } else if line.starts_with("bestmove") {
                println!("Found bestmove: {}", line);
                break;
            }
        }
    }
}
