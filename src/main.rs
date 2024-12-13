use chrono::{DateTime, Duration, TimeDelta, Timelike, Utc};
use clap::{ArgGroup, Parser};
use goesdown::goesimages;
use reqwest::Client;
use std::{path::Path, str::FromStr, sync::Arc};
use tokio::sync::Semaphore;

/// CLI tool to retrieve images from an API with a specified range
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[clap(group(
    ArgGroup::new("time")
        .required(true)
        .multiple(false)
        .args(&["start", "ago"]),
))]
struct Cli {
    /// Start time for the image range in ISO 8601 format (e.g., 2024-11-30T12:00:00Z)
    #[arg(long, group = "time")]
    start: Option<String>,

    /// Time offset from now in a format like "2d12h20m" (e.g., "2 days 12 hours 20 minutes ago")
    #[arg(long, group = "time")]
    ago: Option<String>,

    /// Duration of the image range in a format like "2d12h20m" (optional; defaults to now - start)
    #[arg(short, long)]
    duration: Option<String>,

    /// Time stride for the images in minutes (default: 10)
    #[arg(short, long, default_value = "10")]
    stride: i64,

    /// Root directory to save images (default: current working directory)
    #[arg(short, long, default_value = ".")]
    root: String,

    /// Maximum number of parallel threads (default: 8)
    #[arg(short, long, default_value = "8")]
    max_threads: usize,
}

impl Cli {
    fn validate_and_parse(
        &self,
    ) -> Result<
        (
            impl Iterator<Item = DateTime<Utc>>,
            DateTime<Utc>,
            DateTime<Utc>,
            i64,
        ),
        String,
    > {
        let current_time = Utc::now();

        // Parse start time or calculate it using "ago"
        let start_time = match (&self.start, &self.ago) {
            (Some(start), None) => DateTime::<Utc>::from_str(start)
                .map_err(|e| format!("Invalid start time: {}", e))?,
            (None, Some(ago)) => {
                let duration = parse_duration(ago)?;
                let time = current_time - duration;
                round_to_previous_10_minutes(time)
            }
            _ => return Err("You must specify either --start or --ago, but not both".to_string()),
        };

        // Validate time range
        if current_time - start_time > Duration::days(5) {
            return Err("Start time is too far in the past (maximum range is 5 days)".to_string());
        }

        // Parse or calculate duration
        let end_time = match &self.duration {
            Some(dur) => round_to_previous_10_minutes(start_time + parse_duration(dur)?),
            None => round_to_previous_10_minutes(current_time),
        };

        if end_time > current_time {
            return Err(format!(
                "End time ({}) is in the future (current time {})",
                end_time, current_time
            ));
        }

        if self.stride % 10 != 0 {
            return Err(format!("Stride ({}) must be a multiple of 10", self.stride));
        }

        // if (end_time - start_time).num_minutes() % self.stride != 0 {
        //     return Err(format!(
        //         "Duration ({}) must be a multiple of the stride ({})",
        //         self.duration,
        //         self.stride
        //     ));
        // }

        let time_delta = TimeDelta::minutes(self.stride);
        Ok((
            std::iter::successors(Some(start_time), move |&prev| -> Option<DateTime<Utc>> {
                let next = prev + time_delta;
                if next <= end_time {
                    Some(next)
                } else {
                    None
                }
            }),
            start_time,
            end_time,
            self.stride,
        ))
    }

    fn validate_directory(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<String, String> {
        let root_path = Path::new(&self.root);
        if !root_path.exists() {
            return Err(format!(
                "Specified root directory '{}' does not exist",
                self.root
            ));
        }

        let subdirectory_name = format!(
            "images_{}_to_{}_stride_{}m",
            start_time.format("%Y%m%dT%H%M%S"),
            end_time.format("%Y%m%dT%H%M%S"),
            self.stride
        );
        let subdirectory_path = root_path.join(&subdirectory_name);

        if subdirectory_path.exists() {
            return Err(format!(
                "Subdirectory '{}' already exists",
                subdirectory_path.display()
            ));
        }

        std::fs::create_dir(&subdirectory_path).map_err(|e| {
            format!(
                "Failed to create subdirectory '{}': {}",
                subdirectory_path.display(),
                e
            )
        })?;

        Ok(subdirectory_path.to_string_lossy().to_string())
    }
}

fn parse_duration(input: &str) -> Result<Duration, String> {
    let mut total_minutes = 0;
    let mut value = String::new();

    for c in input.chars() {
        if c.is_digit(10) {
            value.push(c);
        } else {
            let num: i64 = value.parse().map_err(|_| "Invalid duration value")?;
            value.clear();
            total_minutes += match c {
                'm' => num,
                'h' => num * 60,
                'd' => num * 1440,
                _ => return Err("Unsupported duration unit. Use m, h, or d".to_string()),
            };
        }
    }

    if total_minutes % 10 != 0 {
        return Err("Duration must be a multiple of 10 minutes".to_string());
    }

    Ok(Duration::minutes(total_minutes))
}

fn round_to_previous_10_minutes(dt: DateTime<Utc>) -> DateTime<Utc> {
    let rounded_minutes = (dt.minute() / 10) * 10;
    dt.with_minute(rounded_minutes)
        .unwrap()
        .with_second(0)
        .unwrap()
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.validate_and_parse() {
        Ok((time_iter, start_time, end_time, stride)) => {
            match cli.validate_directory(start_time, end_time) {
                Ok(subdirectory) => {
                    println!("Created subdirectory: {}", subdirectory);

                    println!(
                        "Fetching images from {} to {} with a stride of {} minutes",
                        start_time, end_time, stride
                    );

                    let client = Client::new();
                    let semaphore = Arc::new(Semaphore::new(cli.max_threads));

                    let tasks: Vec<_> = time_iter
                        .map(|time| {
                            let permit = semaphore.clone().acquire_owned();
                            let client = client.clone();
                            let subdirectory = subdirectory.clone();

                            tokio::spawn(async move {
                                let _permit = permit.await.unwrap();
                                fetch_image(client, subdirectory, time).await
                            })
                        })
                        .collect();

                    for task in tasks {
                        match task.await {
                            Ok(Ok(path)) => println!("Saved image to {}", path),
                            Ok(Err(e)) => eprintln!("Error fetching image: {}", e),
                            Err(e) => eprintln!("Task panicked: {}", e),
                        }
                    }
                }
                Err(e) => eprintln!("Directory Error: {}", e),
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

async fn fetch_image(
    client: Client,
    subdirectory: String,
    time: DateTime<Utc>,
) -> Result<String, String> {
    let url = goesimages::construct_image_url(&goesimages::Sat::GoesEast, &time)
        .map_err(|e| format!("Failed to construct url for time {time}: {e}"))?;

    let response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| format!("Failed to fetch {url}: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch {url}: HTTP{}", response.status()));
    }

    let image_path = format!("{subdirectory}/{}.jpg", time.format("%Y%m%dT%H%M%S"));
    let bytes = response.bytes().await.map_err(|e| format!("Failed to read response: {e}"))?;
    tokio::fs::write(&image_path, bytes).await.map_err(|e| format!("Failed to save image: {e}"))?;

    Ok(image_path)
}
