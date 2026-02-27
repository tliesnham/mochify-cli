mod api;
mod cli;
mod mcp;

use anyhow::Result;
use api::{MochifyClient, ProcessParams};
use clap::Parser;
use cli::{Args, Commands};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Commands::Serve) => run_mcp_server(args.api_key).await,
        None => {
            if args.files.is_empty() {
                eprintln!("No input files specified. Run with --help for usage.");
                std::process::exit(1);
            }
            process_files(args).await
        }
    }
}

async fn process_files(args: Args) -> Result<()> {
    let client = MochifyClient::new(args.api_key);
    let params = ProcessParams {
        format: args.format,
        width: args.width,
        height: args.height,
        crop: if args.crop { Some(true) } else { None },
        rotation: args.rotation,
    };

    for file_path in &args.files {
        let out_dir = match &args.output {
            Some(d) => d.clone(),
            None => file_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".")),
        };

        match client.squish(file_path, &params, &out_dir).await {
            Ok(out) => println!("{}", out.display()),
            Err(e) => eprintln!("Error processing {}: {e:#}", file_path.display()),
        }
    }

    Ok(())
}

async fn run_mcp_server(api_key: Option<String>) -> Result<()> {
    use rmcp::ServiceExt;

    let server = mcp::MochifyMcp::new(api_key)
        .serve(rmcp::transport::stdio())
        .await?;
    server.waiting().await?;
    Ok(())
}
