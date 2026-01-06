//! Output writers for simulation results.

use crate::metrics::{CsvSummaryRow, GameMetrics};
use crate::types::OutputFormat;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

pub struct OutputWriter {
    jsonl_writer: Option<Box<dyn Write + Send>>,
    csv_writer: Option<csv::Writer<BufWriter<File>>>,
    jsonl_path: Option<PathBuf>,
    csv_path: Option<PathBuf>,
}

impl OutputWriter {
    pub fn new(
        output_dir: &str,
        format: &OutputFormat,
        compress: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let dir = Path::new(output_dir);
        std::fs::create_dir_all(dir)?;

        let timestamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_else(|_| "unknown".to_string())
            .replace(':', "-");

        // Create JSONL writer
        let (jsonl_writer, jsonl_path) = if matches!(format, OutputFormat::Jsonl) {
            let filename = format!("simulation_{}.jsonl", timestamp);
            let path = dir.join(&filename);

            let (writer, final_path) = if compress {
                let gz_path = dir.join(format!("{}.gz", filename));
                let writer: Box<dyn Write + Send> = Box::new(BufWriter::new(GzEncoder::new(
                    File::create(&gz_path)?,
                    Compression::default(),
                )));
                (Some(writer), Some(gz_path))
            } else {
                let file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&path)?;
                let writer: Box<dyn Write + Send> = Box::new(BufWriter::new(file));
                (Some(writer), Some(path))
            };
            (writer, final_path)
        } else {
            (None, None)
        };

        // Always create CSV summary
        let csv_filename = format!("simulation_{}_summary.csv", timestamp);
        let csv_path = dir.join(&csv_filename);
        let csv_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&csv_path)?;
        let mut csv_writer = csv::Writer::from_writer(BufWriter::new(csv_file));
        csv_writer.write_record(&[
            "game_id", "seed", "winner", "seat0_score", "seat1_score", "seat2_score",
            "seat3_score", "seat0_ai", "seat1_ai", "seat2_ai", "seat3_ai",
        ])?;

        Ok(Self {
            jsonl_writer,
            csv_writer: Some(csv_writer),
            jsonl_path,
            csv_path: Some(csv_path),
        })
    }

    pub fn write_game(&mut self, metrics: &GameMetrics) -> Result<(), Box<dyn std::error::Error>> {
        // Write JSONL line
        if let Some(ref mut writer) = self.jsonl_writer {
            let json = serde_json::to_string(metrics)?;
            writeln!(writer, "{}", json)?;
            writer.flush()?;
        }

        // Write CSV row
        if let Some(ref mut writer) = self.csv_writer {
            let row: CsvSummaryRow = metrics.into();
            writer.serialize(&row)?;
            writer.flush()?;
        }

        Ok(())
    }

    pub fn finish(mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut writer) = self.jsonl_writer {
            writer.flush()?;
        }
        if let Some(ref mut writer) = self.csv_writer {
            writer.flush()?;
        }
        Ok(())
    }

    pub fn output_paths(&self) -> (Option<&PathBuf>, Option<&PathBuf>) {
        (self.jsonl_path.as_ref(), self.csv_path.as_ref())
    }
}

