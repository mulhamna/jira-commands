use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use jira_mcp::{run_stdio, run_streamable_http};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Parser)]
#[command(
    name = "jirac-mcp",
    about = "MCP server for Jira powered by jira-core",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run the MCP server over stdio or Streamable HTTP
    Serve {
        #[arg(long, value_enum, default_value = "stdio")]
        transport: Transport,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 8787)]
        port: u16,
        #[arg(long, default_value = "/mcp")]
        path: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum Transport {
    Stdio,
    StreamableHttp,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    match cli.command {
        Commands::Serve {
            transport,
            host,
            port,
            path,
        } => match transport {
            Transport::Stdio => run_stdio().await?,
            Transport::StreamableHttp => {
                let normalized_path = if path.starts_with('/') {
                    path
                } else {
                    format!("/{path}")
                };
                run_streamable_http(&host, port, &normalized_path).await?;
            }
        },
    }

    Ok(())
}
