use std::{sync::{Arc, RwLock, mpsc::{Receiver, Sender}}, time::{Duration, Instant}};

pub enum IePolWatchType {
	PartyHeaderText(String),
}

pub type EventReceiver = Receiver<Arc<ie::OwnedImage>>;
type Watching = Arc<RwLock<Vec<(IePolWatchType, Sender<Arc<ie::OwnedImage>>)>>>;

#[derive(Clone)]
pub struct IePol {
	next_pol: Arc<RwLock<Instant>>,
	watching: Watching,
}

impl IePol {
	pub fn new(ie: Arc<ie::Ie>) -> Self {
		let next_pol = Arc::new(RwLock::new(Instant::now()));
		let watching = Watching::default();
		let _thread = {
			let next_pol = next_pol.clone();
			let watching = watching.clone();
			std::thread::spawn(move || -> Result<(), anyhow::Error> {
				loop {
					let next = *next_pol.read().unwrap();
					let now = Instant::now();
					if next > now {
						// std::thread::sleep(next.duration_since(now));
						std::thread::sleep(Duration::from_millis(50));
						continue;
					}
					
					's: {
						let Some(image) = crate::capture::capture() else {break 's};
						let header_text = ie.util_party_header_text(image.as_image()).to_ascii_lowercase();
						let image = Arc::new(image);
						
						println!("header text: {header_text}");
						
						for (typ, tx) in watching.read().unwrap().iter() {
							match typ {
								IePolWatchType::PartyHeaderText(text) if matches(&header_text, &text.to_ascii_lowercase(), 3) =>
									_ = tx.send(image.clone()),
								
								_ => continue
							}
						}
					}
					
					let next = Instant::now() + Duration::from_secs_f32(crate::config().pol_delay);
					if next > *next_pol.read().unwrap() {
						*next_pol.write().unwrap() = next;
					}
				}
			})
		};
		
		Self {
			next_pol,
			watching,
		}
	}
	
	pub fn delay_till(&self, time: Instant) {
		// if time > *self.next_pol.read().unwrap() {
		if time > Instant::now() {
			*self.next_pol.write().unwrap() = time;
		}
	}
	
	pub fn watch_event(&self, typ: IePolWatchType, tx: Sender<Arc<ie::OwnedImage>>) {
		self.watching
			.write()
			.unwrap()
			.push((typ, tx));
	}
	
	pub fn secs_till_next_poll(&self) -> f32 {
		let next = *self.next_pol.read().unwrap();
		let now = Instant::now();
		if next > now {
			return next.duration_since(now).as_secs_f32();
		}
		
		0.0
	}
}

fn matches(a: &str, b: &str, thresshold: usize) -> bool {
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
	
	levenshtein::levenshtein(a, b) <= thresshold
}