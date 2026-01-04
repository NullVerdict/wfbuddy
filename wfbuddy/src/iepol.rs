use std::{
	sync::{
		mpsc::{Receiver, Sender},
		Arc, Condvar, Mutex,
	},
	time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub enum IePolWatchType {
	/// Lowercased string we want to match against the Party Header Text.
	PartyHeaderText(String),
}

pub type EventReceiver = Receiver<Arc<ie::OwnedImage>>;

type Watching = Arc<Mutex<Vec<(IePolWatchType, Sender<Arc<ie::OwnedImage>>)>>>;
type Schedule = Arc<(Mutex<Instant>, Condvar)>;

#[derive(Clone)]
pub struct IePol {
	next_pol: Schedule,
	watching: Watching,
}

impl IePol {
	pub fn new(ie: Arc<ie::Ie>) -> Self {
		let next_pol: Schedule = Arc::new((Mutex::new(Instant::now()), Condvar::new()));
		let watching: Watching = Arc::new(Mutex::new(Vec::new()));

		let next_pol_thread = next_pol.clone();
		let watching_thread = watching.clone();

		std::thread::spawn(move || {
			// NOTE: this is a best-effort background worker.
			// Any failure should log and keep going.
			loop {
				// 1) Wait until it's time to poll (or until someone updates the schedule).
				{
					let (lock, cv) = &*next_pol_thread;
					let mut next = lock.lock().expect("next_pol lock poisoned");
					loop {
						let now = Instant::now();
						if *next <= now {
							break;
						}
						let dur = next.saturating_duration_since(now);
						let (guard, _timeout) = cv
							.wait_timeout(next, dur)
							.expect("next_pol lock poisoned during wait");
						next = guard;
					}
				}

				// 2) Do the expensive part without holding locks.
				if let Some(image) = crate::capture::capture() {
					let header_text = ie.util_party_header_text(image.as_image()).to_ascii_lowercase();
					let image = Arc::new(image);

					// Snapshot watchers so sending can't block the watcher lock.
					let watchers = {
						watching_thread
							.lock()
							.expect("watching lock poisoned")
							.clone()
					};

					for (typ, tx) in watchers {
						match typ {
							IePolWatchType::PartyHeaderText(text)
								if matches(&header_text, &text, 3) => {
									let _ = tx.send(image.clone());
								}
							_ => {}
						}
					}
				}

				// 3) Schedule the next poll.
				let pol_delay = crate::config_read().pol_delay;
				let candidate = Instant::now() + Duration::from_secs_f32(pol_delay);
				let (lock, cv) = &*next_pol_thread;
				let mut next = lock.lock().expect("next_pol lock poisoned");
				if candidate > *next {
					*next = candidate;
				}
				cv.notify_all();
			}
		});

		Self { next_pol, watching }
	}

	pub fn delay_till(&self, time: Instant) {
		let (lock, cv) = &*self.next_pol;
		let mut next = lock.lock().expect("next_pol lock poisoned");
		if time > *next {
			*next = time;
			cv.notify_all();
		}
	}

	pub fn watch_event(&self, typ: IePolWatchType, tx: Sender<Arc<ie::OwnedImage>>) {
		let typ = match typ {
			IePolWatchType::PartyHeaderText(text) => {
				IePolWatchType::PartyHeaderText(text.to_ascii_lowercase())
			}
		};

		self.watching
			.lock()
			.expect("watching lock poisoned")
			.push((typ, tx));
	}

	pub fn secs_till_next_poll(&self) -> f32 {
		let (lock, _) = &*self.next_pol;
		let next = *lock.lock().expect("next_pol lock poisoned");
		let now = Instant::now();
		if next > now {
			return next.duration_since(now).as_secs_f32();
		}
		0.0
	}
}

fn matches(a: &str, b: &str, threshold: usize) -> bool {
	if a == b {
		return true;
	}

	let mut end = a.len();
	while let Some(index) = a[..end].rfind(' ') {
		end = index;
		let sub = &a[..end];
		if sub == b {
			return true;
		}
	}

	levenshtein::levenshtein(a, b) <= threshold
}
