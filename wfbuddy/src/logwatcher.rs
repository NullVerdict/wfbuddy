use std::{fs::File, io::{Read, Seek}, path::PathBuf, sync::{mpsc::{Receiver, Sender}, Arc, RwLock}, time::{Duration, Instant}};
use notify::Watcher;
use regex::Regex;
use crate::util::OwnedCaptures;

const MIN_DELAY_MS: u64 = 100;

pub type EventReceiver = Receiver<(Instant, OwnedCaptures)>;
type Watching = Arc<RwLock<Vec<(Regex, Sender<(Instant, OwnedCaptures)>)>>>;

#[derive(Debug)]
pub enum LogWatcherStatus<'a> {
	Watching,
	Stopped,
	Failed(&'a anyhow::Error),
}

enum LogWatcherThread<R> {
	Handle(std::thread::JoinHandle<R>),
	Joined(R),
	Error(anyhow::Error),
}

pub struct LogWatcher {
	watching: Watching,
	thread: LogWatcherThread<Result<(), anyhow::Error>>,
}

impl LogWatcher {
	pub fn watch(log_file: impl Into<PathBuf>) -> Result<Self, anyhow::Error> {
		let line_regex = Regex::new(r"^(?<time>[\d\.]+) (?<rest>.+)$").unwrap();
		
		let log_file = log_file.into();
		let watching = Watching::default();
		let thread = {
			let watching = watching.clone();
			std::thread::spawn(move || -> Result<(), anyhow::Error> {
				let (tx, rs) = std::sync::mpsc::channel();
				let mut watcher = notify::RecommendedWatcher::new(tx, Default::default())?;
				watcher.watch(&log_file, notify::RecursiveMode::NonRecursive)?;
				
				let mut position = File::open(&log_file)?.seek(std::io::SeekFrom::End(0))?;
				// warframe can log in batches, splitting up lines, this way we will always have whole lines
				let mut partial_line = String::new();
				let mut start = Instant::now();
				
				while let Ok(Ok(event)) = rs.recv() {
					if event.kind != notify::EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any)) {continue};
					// println!("received change {event:?}");
					
					let mut f = File::open(&log_file)?;
					let len = f.seek(std::io::SeekFrom::End(0))?;
					if len == 0 {continue}
					
					if len < position {
						log::info!("reset log position {len} {position}");
						position = 0;
					}
					
					f.seek(std::io::SeekFrom::Start(position))?;
					let mut buf = Vec::new();
					let read_count = f.read_to_end(&mut buf)?;
					position += read_count as u64;
					let Ok(s) = str::from_utf8(&buf[..read_count]) else {
						log::warn!("Failed converting read data into string");
						continue;
					};
					
					let watching = watching.read().unwrap();
					
					partial_line.push_str(&s);
					let mut index = 0;
					// fucking hate you return carriage, what year is it, 19th century?
					while let Some(offset) = partial_line[index..].find("\r\n") {
						'c: {
							let line = &partial_line[index..index + offset];
							// println!("log line: {line}");
							let Some(cap) = line_regex.captures(line) else {break 'c};
							let Ok(offset_time) = cap["time"].parse::<f32>() else {break 'c};
							let rest = &cap["rest"];
							// println!("{offset_time} {rest}");
							let offset_duration = Duration::from_secs_f32(offset_time);
							let estimated_start = Instant::now() - offset_duration;
							if estimated_start < start {
								start = estimated_start;
								log::debug!("new estimated start {start:?}");
							}
							
							for (regex, wtx) in watching.iter() {
								// println!("testing\n\t`{rest}`\nagainst\n\t{regex:?}");
								let Some(cap2) = regex.captures(rest) else {continue};
								// println!("{regex:?} {offset_time} {rest}");
								_ = wtx.send((start + offset_duration, (regex, cap2).into()));
							}
						}
						
						index += offset + 2;
					}
					
					if index >= partial_line.len() {
						partial_line.clear();
					} else {
						partial_line = partial_line[index..].to_string();
					}
					
					std::thread::sleep(Duration::from_millis(MIN_DELAY_MS));
				}
				
				log::info!("ending watch");
				
				Ok(())
			})
		};
		
		Ok(Self {
			watching,
			thread: LogWatcherThread::Handle(thread),
		})
	}
	
	pub fn watch_event(&self, regex: Regex, tx: Sender<(Instant, OwnedCaptures)>) {
		self.watching
			.write()
			.unwrap()
			.push((regex, tx));
	}
	
	pub fn status<'a>(&'a mut self) -> LogWatcherStatus<'a> {
		if let LogWatcherThread::Handle(handle) = &self.thread {
			if !handle.is_finished() {
				return LogWatcherStatus::Watching;
			}
			
			// We do unsafe code since std::mem::take, swap, and replace need existing values or a default, which we dont have yet
			unsafe {
				let thread_ptr = &mut self.thread as *mut _;
				let thread = std::ptr::read(thread_ptr);
				let LogWatcherThread::Handle(handle) = thread else {unreachable!()};
				match handle.join() {
					Ok(v) => thread_ptr.write(LogWatcherThread::Joined(v)),
					Err(e) => thread_ptr.write(LogWatcherThread::Error(anyhow::Error::msg(format!("Thread panic {e:#?}")))),
				}
			}
		}
		
		match &self.thread {
			LogWatcherThread::Handle(_) => unreachable!(),
			
			LogWatcherThread::Joined(r) => match r {
				Ok(_) => LogWatcherStatus::Stopped,
				Err(e) => LogWatcherStatus::Failed(e),
			}
			
			LogWatcherThread::Error(e) => LogWatcherStatus::Failed(e)
		}
	}
}