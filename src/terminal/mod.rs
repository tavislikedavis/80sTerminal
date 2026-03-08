pub mod grid;
mod parser;
mod pty;
mod selection;

pub use grid::{Cell, CellColor, CellStyle, Grid};
pub use parser::Parser;
pub use selection::{Point, Selection};

use crate::config::Config;
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::{error, info};
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::Arc;
use std::thread;

pub struct Terminal {
    grid: Arc<Mutex<Grid>>,
    parser: Parser,
    writer: Box<dyn Write + Send>,
    data_receiver: Receiver<Vec<u8>>,
    _reader_handle: thread::JoinHandle<()>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    cols: u16,
    rows: u16,
    pty_master: Box<dyn portable_pty::MasterPty + Send>,
}

impl Terminal {
    pub fn new(cols: u16, rows: u16, config: &Config) -> anyhow::Result<Self> {
        let profile = config.get_profile("default");
        let scrollback = profile.scrollback_lines;

        let pty_system = native_pty_system();
        let pty_pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(&profile.shell);
        cmd.arg("-l"); // Login shell: sources .zprofile/.zshrc for proper PATH
        cmd.cwd(shellexpand::tilde(&profile.working_directory).to_string());

        // Set environment variables
        for (key, value) in &profile.environment {
            cmd.env(key, value);
        }

        // Set TERM
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let child = pty_pair.slave.spawn_command(cmd)?;

        let mut grid = Grid::new(cols, rows, scrollback);
        grid.set_unlimited_scrollback(profile.unlimited_scrollback);
        let grid = Arc::new(Mutex::new(grid));
        let parser = Parser::new();

        let mut reader = pty_pair.master.try_clone_reader()?;
        let writer = pty_pair.master.take_writer()?;

        // Channel for passing data from reader thread to main thread
        let (data_sender, data_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = unbounded();

        // Spawn reader thread that just reads data and sends it through channel
        let reader_handle = thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        info!("PTY closed");
                        break;
                    }
                    Ok(n) => {
                        if data_sender.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Read error: {}", e);
                        break;
                    }
                }
            }
        });

        info!("Terminal created: {}x{}", cols, rows);

        Ok(Self {
            grid,
            parser,
            writer,
            data_receiver,
            _reader_handle: reader_handle,
            _child: child,
            cols,
            rows,
            pty_master: pty_pair.master,
        })
    }

    pub fn write(&mut self, data: &[u8]) {
        if let Err(e) = self.writer.write_all(data) {
            error!("Write error: {}", e);
        }
        let _ = self.writer.flush();
    }

    pub fn process(&mut self) {
        // Limit bytes processed per frame to keep the render loop responsive.
        // A 50MB cat would otherwise block here for seconds/minutes.
        const MAX_BYTES_PER_FRAME: usize = 256 * 1024; // 256KB per frame
        let mut bytes_processed = 0;
        let mut had_data = false;

        let mut grid = self.grid.lock();
        while let Ok(data) = self.data_receiver.try_recv() {
            bytes_processed += data.len();
            had_data = true;
            self.parser.process(&data, &mut grid);
            if bytes_processed >= MAX_BYTES_PER_FRAME {
                break;
            }
        }

        // Snap back to bottom when new output arrives
        if had_data && grid.scroll_offset() > 0 {
            grid.set_scroll_offset(0);
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows {
            return;
        }

        self.cols = cols;
        self.rows = rows;

        let _ = self.pty_master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });

        self.grid.lock().resize(cols, rows);
        info!("Terminal resized to {}x{}", cols, rows);
    }

    pub fn grid(&self) -> &Arc<Mutex<Grid>> {
        &self.grid
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }
}
