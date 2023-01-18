use std::io::{stdout, Write, Stdout};

use tokio::sync::mpsc::{Receiver, Sender};

use crate::server_state::ServerData;
use crossterm::{cursor::*, style::*, QueueableCommand, Result, terminal::{Clear, ClearType}};

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Mark this chunk as downloaded
    ChunkDownloaded(u64, u64),
    /// Mark this chunk as requested
    ChunkRequested(u64, u64),
    /// Mark this file as done
    FileDone(u64),
}

const MAX_FILE_NAME_LEN: usize = 20;

pub struct ProgressIndicator {
    stream: Stdout,
    event_rx: Receiver<ProgressEvent>,
    event_tx: Sender<ProgressEvent>,
    file_lengths: Vec<u64>,
    file_names: Vec<String>,
    done_files: Vec<bool>,
}

impl ProgressIndicator {
    /// Create a new progress indicator
    pub fn new(server_data: &ServerData) -> Self {
        let file_lengths = server_data.files.iter().map(|f| f.1.num_chunks).collect();
        let file_names = server_data.files.iter().map(|f| f.0.path.clone()).collect();

        let (event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        Self {
            stream: stdout(),
            event_rx,
            event_tx,
            file_lengths,
            file_names,
            done_files: vec![false; server_data.files.len()],

        }
    }

    /// Get the event sender
    pub fn event_tx(&self) -> Sender<ProgressEvent> {
        self.event_tx.clone()
    }

    /// Run the progress indicator
    pub async fn run(&mut self) -> Result<()> {
        self.init_progress_bars().expect("Failed to initialise progress bars");
        loop {
            let event = self.event_rx.recv().await.unwrap();
            match event {
                ProgressEvent::ChunkDownloaded(file_idx, chunk_idx) => {
                    let file_name_len_part = self.file_names[file_idx as usize].len().min(MAX_FILE_NAME_LEN) + 3; // ": ["
                    self.stream.queue(MoveToNextLine(file_idx as u16 + 1))?;
                    self.stream.queue(MoveToColumn(chunk_idx as u16 + file_name_len_part as u16))?;
                    self.stream.queue(PrintStyledContent("X".green()))?;
                    self.stream.queue(MoveToPreviousLine(file_idx as u16 + 1))?;
                    self.stream.queue(Clear(ClearType::CurrentLine))?;
                    self.stream.flush().unwrap();
                },
                ProgressEvent::ChunkRequested(file_idx, chunk_idx) => {
                    let file_name_len_part = self.file_names[file_idx as usize].len().min(MAX_FILE_NAME_LEN) + 3; // ": ["
                    self.stream.queue(MoveToNextLine(file_idx as u16 + 1))?;
                    self.stream.queue(MoveToColumn(chunk_idx as u16 + file_name_len_part as u16))?;
                    self.stream.queue(PrintStyledContent("?".yellow()))?;
                    self.stream.queue(MoveToPreviousLine(file_idx as u16 + 1))?;
                    self.stream.queue(Clear(ClearType::CurrentLine))?;
                    self.stream.flush().unwrap();
                },
                ProgressEvent::FileDone(file_idx) => {
                    self.stream.queue(MoveToNextLine(file_idx as u16 + 1))?;
                    self.stream.queue(Print(self.file_names[file_idx as usize].clone()))?;
                    self.stream.queue(PrintStyledContent(" Done".green()))?;
                    self.stream.queue(Clear(ClearType::UntilNewLine))?;
                    self.stream.queue(MoveToPreviousLine(file_idx as u16 + 1))?;
                    self.stream.queue(Clear(ClearType::CurrentLine))?;
                    self.stream.flush().unwrap();

                    self.done_files[file_idx as usize] = true;
                    if self.done_files.iter().all(|f| *f) {
                        break Ok(());
                    }
                },
            }
        }
    }

    /// Initialise the progress bars
    fn init_progress_bars(&mut self) -> Result<()> {
        // Leave enough blank lines for the progress bars
        for _ in 0..self.file_names.len() {
            self.stream.queue(Print('\n'))?;
        }
        self.stream.queue(MoveToPreviousLine(self.file_names.len() as u16))?;
        for (file_idx, file) in self.file_names.iter().enumerate() {
            self.stream.queue(Clear(ClearType::UntilNewLine))?;
            let file_len = self.file_lengths[file_idx];
            let first_name_part = file[..file.len().min(MAX_FILE_NAME_LEN)].to_string();
            self.stream.queue(Print(format!("{}: ", first_name_part)))?;
            self.stream.queue(PrintStyledContent("[".blue()))?;
            for _ in 0..file_len {
                self.stream.queue(Print("."))?;
            }
            self.stream.queue(PrintStyledContent("]".blue()))?;
            self.stream.queue(MoveToNextLine(1))?;
        }
        self.stream.queue(MoveToPreviousLine(self.file_names.len() as u16 + 1))?;
        self.stream.flush()?;
        Ok(())
    }
}