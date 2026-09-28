use agenthub_core::{
    canonical, git, models::Target, planner, scanner, secrets, transaction, AgentHub,
};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::{self, Read};

#[derive(Parser)]
#[command(
    name = "agenthub",
    version,
    about = "Skills · MCP · Plugins · Rules canonical manager"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        import_all: bool,
        #[arg(long)]
        empty: bool,
    },
    Scan {
        #[arg(long, default_value = "all")]
        target: String,
    },
    Inventory {
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Target {
        #[command(subcommand)]
        command: TargetCommand,
    },
    Plan {
        target: TargetArg,
        #[arg(long)]
        json: bool,
    },
    Sync {
        target: TargetArg,
        #[arg(long)]
        plan_id: String,
        #[arg(long)]
        confirm: bool,
    },
    Rollback {
        transaction_id: String,
    },
    History {
        #[arg(long)]
        json: bool,
    },
    Git {
        #[command(subcommand)]
        command: GitCommand,
    },
    Secret {
        #[command(subcommand)]
        command: SecretCommand,
    },
    Doctor {
        #[arg(long)]
        json: bool,
    },
}
#[derive(Subcommand)]
enum TargetCommand {
    Enable { target: TargetArg },
    Disable { target: TargetArg },
}
#[derive(Subcommand)]
enum GitCommand {
    Status,
    Diff,
    Commit {
        #[arg(short, long)]
        message: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        email: Option<String>,
    },
    Log,
    Restore {
        commit: String,
        #[arg(long)]
        capability: Option<String>,
    },
}
#[derive(Subcommand)]
enum SecretCommand {
    Set { name: String },
}
#[derive(Clone)]
struct TargetArg(Target);
impl std::str::FromStr for TargetArg {
    type Err = anyhow::Error;
    fn from_str(v: &str) -> Result<Self> {
        Ok(Self(v.parse()?))
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let hub = AgentHub::open_default()?;
    match cli.command {
        Command::Init { import_all, empty } => {
            anyhow::ensure!(
                !(import_all && empty),
                "choose either --import-all or --empty"
            );
            git::ensure_repo(&hub.paths.root)?;
            let mut found = scanner::scan_global(&hub.paths, &Target::ALL)?;
            if import_all {
                for i in &mut found {
                    i.selected = true;
                }
                let imported = canonical::import_scan_items(&hub.paths, &found)?;
                hub.store.set_initialized(true)?;
                println!("imported {} capabilities", imported.len());
            } else if empty {
                hub.store.set_initialized(true)?;
                println!("initialized an empty Canonical source");
            } else {
                println!("{}", serde_json::to_string_pretty(&found)?);
                eprintln!("Review the discovery result, then use the desktop selection flow, --import-all, or --empty. Initialization is not complete.");
            }
        }
        Command::Scan { target } => {
            let targets = parse_targets(&target)?;
            if hub.store.initialized()? {
                let reports: Vec<_> = targets
                    .into_iter()
                    .map(|t| planner::create(&hub.paths, t))
                    .collect::<Result<_>>()?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({"mode":"drift_audit","targets":reports})
                    )?
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&scanner::scan_global(&hub.paths, &targets)?)?
                );
            }
        }
        Command::Inventory { kind, json } => {
            let mut inv = canonical::inventory(&hub.paths)?;
            if let Some(k) = kind {
                inv.retain(|v| v.kind.as_str() == k);
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&inv)?)
            } else {
                for c in inv {
                    println!("{}\t{}\t{}", c.kind.as_str(), c.id, &c.digest[..12]);
                }
            }
        }
        Command::Target { command } => match command {
            TargetCommand::Enable { target } => hub.store.set_target(target.0, true)?,
            TargetCommand::Disable { target } => hub.store.set_target(target.0, false)?,
        },
        Command::Plan { target, json: _ } => {
            let p = planner::create(&hub.paths, target.0)?;
            hub.store.save_plan(&p)?;
            println!("{}", serde_json::to_string_pretty(&p)?);
        }
        Command::Sync {
            target,
            plan_id,
            confirm,
        } => {
            anyhow::ensure!(confirm, "--confirm is required");
            let p = hub.store.plan(&plan_id)?.context("plan not found")?;
            anyhow::ensure!(p.target == target.0, "plan target mismatch");
            let tx = transaction::apply(&hub.paths, &hub.store, &p)?;
            println!("{}", serde_json::to_string_pretty(&tx)?);
        }
        Command::Rollback { transaction_id } => println!(
            "{}",
            serde_json::to_string_pretty(&transaction::rollback(
                &hub.paths,
                &hub.store,
                &transaction_id,
            )?)?
        ),
        Command::History { json: _ } => println!(
            "{}",
            serde_json::to_string_pretty(&hub.store.recent_transactions(100)?)?
        ),
        Command::Git { command } => match command {
            GitCommand::Status => println!("{}", git::status(&hub.paths.root)?),
            GitCommand::Diff => println!("{}", git::diff(&hub.paths.root)?),
            GitCommand::Commit {
                message,
                name,
                email,
            } => println!(
                "{}",
                git::commit(&hub.paths.root, &message, name.as_deref(), email.as_deref())?
            ),
            GitCommand::Log => println!("{}", git::log(&hub.paths.root)?),
            GitCommand::Restore { commit, capability } => {
                git::restore(&hub.paths.root, &commit, capability.as_deref())?
            }
        },
        Command::Secret { command } => match command {
            SecretCommand::Set { name } => {
                let mut value = Vec::new();
                io::stdin().read_to_end(&mut value)?;
                while value.last().is_some_and(|b| *b == b'\n' || *b == b'\r') {
                    value.pop();
                }
                secrets::set(&hub.store, &hub.paths, &name, &value)?;
                println!("secret stored");
            }
        },
        Command::Doctor { json } => {
            let report = serde_json::json!({"initialized":hub.store.initialized()?,"root":hub.paths.root,"git":git::snapshot(&hub.paths.root)?,"masterKey":hub.paths.master_key.exists(),"targets":hub.store.enabled_targets()?});
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?)
            } else {
                println!(
                    "AgentHub doctor\n{}",
                    serde_json::to_string_pretty(&report)?
                );
            }
        }
    }
    Ok(())
}
fn parse_targets(value: &str) -> Result<Vec<Target>> {
    if value == "all" {
        Ok(Target::ALL.to_vec())
    } else {
        Ok(vec![value.parse()?])
    }
}
