extern crate clap;
use clap::{arg, command, parser::ValuesRef};

use std::{cmp::min, error::Error};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::io::{Write,Cursor};
use std::path::Path;
use flate2::read::GzDecoder;
use tar::Archive;

use reqwest::Client;
use indicatif::{ProgressBar, ProgressStyle};
use futures_util::StreamExt;

#[macro_use(concat_string)]
extern crate concat_string;

use regex::Regex;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    const WEB_URL: &str = "export.arxiv.org";
    const APP_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; rv:113.0) Gecko/20100101 Firefox/113.0";
    const DL_DIR: &str = "dl";
    const EXTRACTED_DIR: &str = "extracted";
    let matches = command!() // requires `cargo` feature
        .arg(arg!(--dl_dir <DL_DIR>).default_value(DL_DIR).help("directory to download archives to"))
        .arg(arg!(--extract_dir <EXTRACT_DIR>).default_value(EXTRACTED_DIR).help("directory to extract archives to"))
        .arg(arg!(ids: <IDs>).num_args(1..).help("arxiv IDs"))
        .get_matches();

    let dl_dir = matches.get_one::<String>("dl_dir").unwrap();
    let dl_dir = Path::new(&dl_dir);
    let extract_dir = matches.get_one::<String>("extract_dir").unwrap();
    let extract_dir = Path::new(&extract_dir);
    if let Ok(Some(values)) = matches.try_get_many::<String>("ids") {
        let values: Vec<String> = values.map(|s| s.to_string()).collect();
        let client = Client::builder()
            .user_agent(APP_USER_AGENT)
            .build()?;
        for id in values {
            if match_arxiv_id(&id) {
                let path_to_dl = dl_dir.join(&id).display().to_string();
                let url = concat_string!("https://", WEB_URL, "/e-print/", &id);
                let _res = download_file(&client, &url, &path_to_dl).await;

                let path_to_extract = extract_dir.join(&id).display().to_string();

                let tar_gz = fs::File::open(path_to_dl)?;
                let tar = GzDecoder::new(tar_gz);
                let mut archive = Archive::new(tar);
                archive.unpack(&path_to_extract)?;
                println!("Downloaded file extracted to {}", path_to_extract)
            } else {
                println!("Isn't a valid arxiv id: {}", id)
               // Err(InvalidIDError)
            }
        }
    } else {
        println!("Error while processing IDs")
    }
    Ok(())
}

fn match_arxiv_id(string: &str) -> bool {
/* Valid ID strings:
hep-th/9901001
hep-th/9901001v1
math.CA/0611800v2

0704.0001
0704.0001v1
1412.7878
1501.00001
9912.12345v2 
*/
    Regex::new(r"(\w+[.-]\w+/)|(\d+.)\d+v?\d+").unwrap().is_match(string)
}

pub async fn download_file(client: &Client, url: &str, path: &str) -> Result<(), String> {
    // Reqwest setup
    let res = client
        .head(url)
        .send()
        .await
        .or(Err(format!("'{}', HEAD request failed", &url)))?;
    let total_size = res.headers().get("content-length").map_or(0u64, |c| {
        c.to_str().unwrap().parse().unwrap()
    });
    let file_size = fs::metadata(path).map_or(0, |f| f.len());
    if total_size > 0 && file_size == total_size {
        println!("File '{}' is already downloaded", path);
        return Ok(());
    }
    let res = client
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;
    
    // Indicatif setup
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("#>-"));
    pb.set_message(&format!("Downloading {}", url));

    // download chunks
    let mut file = fs::File::create(path).or(Err(format!("Failed to create file '{}'", path)))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;
        file.write_all(&chunk)
            .or(Err(format!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(&format!("Downloaded {} to {}", url, path));
    return Ok(());
}