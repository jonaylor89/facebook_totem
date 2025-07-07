use anyhow::Result;
use clap::{Parser, Subcommand};
use csv::Reader;
use facebook_totem::{get_id_from_url, get_ads_from_id, get_facebook_page_from_name, write_json_to_csv, write_facebook_pages_to_csv};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
    
    #[arg(short, long)]
    output: String,
}

#[derive(Subcommand)]
enum Mode {
    Single {
        #[arg(short, long)]
        url: String,
    },
    Multi {
        #[arg(long)]
        urls: String,
        #[arg(short, long)]
        columns: String,
    },
    Search {
        #[arg(short, long)]
        target: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    if !Path::new("output").exists() {
        std::fs::create_dir("output")?;
    }
    
    match cli.mode {
        Mode::Single { url } => {
            println!("Getting page ID from URL...");
            let id = get_id_from_url(&url).await?;
            println!("Getting ads for page ID: {}", id);
            let result = get_ads_from_id(&id).await?;
            
            if !result.is_empty() {
                let output_path = format!("output/{}", cli.output);
                write_json_to_csv(&result, &output_path)?;
                println!("You can see the output in: {}", output_path);
            } else {
                println!("Sorry, but this page hasn't used any ads");
            }
        }
        Mode::Multi { urls, columns } => {
            let mut targets = Vec::new();
            let mut rdr = Reader::from_path(&urls)?;
            
            for result in rdr.deserialize() {
                let record: HashMap<String, String> = result?;
                if let Some(url) = record.get(&columns) {
                    targets.push(url.clone());
                }
            }
            
            println!("{} targets found", targets.len());
            
            let pb = ProgressBar::new(targets.len() as u64);
            pb.set_style(ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")?
                .progress_chars("##-"));
            
            for target in targets {
                pb.set_message(format!("Processing: {}", target));
                
                let username = target.split(".com/").nth(1)
                    .unwrap_or("")
                    .replace('/', "");
                
                match get_id_from_url(&target).await {
                    Ok(id) => {
                        match get_ads_from_id(&id).await {
                            Ok(result) => {
                                if !result.is_empty() {
                                    let output_path = format!("output/{}{}.csv", username, id);
                                    let _ = write_json_to_csv(&result, &output_path);
                                }
                            }
                            Err(_) => {
                                // Skip failed requests
                            }
                        }
                    }
                    Err(_) => {
                        // Skip failed requests
                    }
                }
                
                pb.inc(1);
            }
            
            pb.finish_with_message("Processing complete");
            println!("You can see the results in the output folder. Pages that used ads have a file in the output folder.");
        }
        Mode::Search { target } => {
            println!("Searching for pages with name: {}", target);
            let result = get_facebook_page_from_name(&target).await?;
            
            if !result.is_empty() {
                let output_path = format!("output/{}", cli.output);
                write_facebook_pages_to_csv(&result, &output_path)?;
                println!("You can see the output in: {}", output_path);
            } else {
                println!("Sorry, no pages found with this name");
            }
        }
    }
    
    Ok(())
}