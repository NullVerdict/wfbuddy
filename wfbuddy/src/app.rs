//! Iced application (Model-View-Update).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iced::widget::{
	button, container, Button, Checkbox, Column, Container, PickList, Row, Scrollable, Text, TextInput,
};
use iced::{Element, Length, Subscription, Task};

use crate::capture::{capture_by_app_name, list_windows, WindowInfo};
use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    RelicRewards,
    Settings,
    Debug,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tab::RelicRewards => write!(f, "Relic Rewards"),
            Tab::Settings => write!(f, "Settings"),
            Tab::Debug => write!(f, "Debug"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(Instant),

    TabSelected(Tab),

    RefreshWindows,
    WindowsLoaded(Result<Vec<WindowInfo>, String>),
    WindowPicked(WindowInfo),

    AppNameChanged(String),
    PollDelayChanged(String),
    MaxCaptureHeightChanged(String),

    SaveConfig,
    ConfigSaved(Result<(), String>),

    SampleTheme,
    ThemeSampled(Result<ie::Theme, String>),

    PollPartyHeaderNow,
    PartyHeaderPolled(Result<Option<String>, String>),

    PollRelicNow,
    RelicPolled(Result<RelicPollResult, String>),

    ValuedFormaToggled(bool),
}

#[derive(Debug, Clone)]
pub struct RelicPollResult {
    pub rewards: ie::screen::relicreward::Rewards,
    pub selected: Option<usize>,
}

#[derive(Debug)]
struct PollState {
    last_party_poll: Instant,
    party_in_flight: bool,

    reward_mode_until: Option<Instant>,
    next_reward_poll: Instant,
    reward_in_flight: bool,
}

impl PollState {
    fn new(now: Instant) -> Self {
        Self {
            last_party_poll: now - Duration::from_secs(60),
            party_in_flight: false,
            reward_mode_until: None,
            next_reward_poll: now,
            reward_in_flight: false,
        }
    }
}

#[derive(Debug, Default)]
struct RelicState {
    rewards: Option<ie::screen::relicreward::Rewards>,
    selected: Option<usize>,
    valued_forma: bool,
    last_updated: Option<Instant>,
}

#[derive(Debug, Default)]
struct DebugState {
    last_party_header: Option<String>,
}

pub struct App {
    tab: Tab,

    config: Config,

    // Editable fields (text inputs)
    app_name_input: String,
    poll_delay_input: String,
    max_capture_height_input: String,

    windows: Vec<WindowInfo>,
    selected_window: Option<WindowInfo>,

    status: Option<String>,

    ie: Arc<Mutex<ie::Ie>>,
    data: Option<data::Data>,

    poll: PollState,
    relic: RelicState,
    debug: DebugState,
}

pub fn run() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .subscription(App::subscription)
        .run()
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let cfg = Config::load_or_default();

        let detection = resolve_model_path("ocr/detection.mnn");
        let recognition = resolve_model_path("ocr/latin_recognition.mnn");
        let charsset = resolve_model_path("ocr/latin_charset.txt");

        let ie = ie::Ie::new(detection, recognition, charsset, cfg.theme);
        let ie = Arc::new(Mutex::new(ie));

        // Data loading can fail (network/offline). We keep the app usable without it.
        let data = match data::Data::populated(data::Language::English) {
            Ok(d) => Some(d),
            Err(err) => {
                tracing::warn!(error = %err, "failed to load data; ducat/vaulted info disabled");
                None
            }
        };

        let windows = match list_windows() {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(error = %err, "failed to enumerate windows");
                vec![]
            }
        };

        let selected_window = windows
            .iter()
            .find(|w| w.app_name == cfg.app_name)
            .cloned();

        let now = Instant::now();

        let app = Self {
            tab: Tab::RelicRewards,
            app_name_input: cfg.app_name.clone(),
            poll_delay_input: cfg.poll_delay_s.to_string(),
            max_capture_height_input: cfg
                .max_capture_height
                .map(|v| v.to_string())
                .unwrap_or_default(),

            windows,
            selected_window,

            status: None,
            ie,
            data,

            poll: PollState::new(now),
            relic: RelicState::default(),
            debug: DebugState::default(),
            config: cfg,
        };

        (app, Task::none())
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(250)).map(Message::Tick)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => self.on_tick(now),

            Message::TabSelected(tab) => {
                self.tab = tab;
                Task::none()
            }

            Message::RefreshWindows => Task::perform(
                async { list_windows().map_err(|e| e.to_string()) },
                Message::WindowsLoaded,
            ),

            Message::WindowsLoaded(res) => {
                match res {
                    Ok(v) => {
                        self.windows = v;
                        self.selected_window = self
                            .windows
                            .iter()
                            .find(|w| w.app_name == self.config.app_name)
                            .cloned();
                        self.status = Some("Window list refreshed.".into());
                    }
                    Err(err) => {
                        self.status = Some(format!("Failed to list windows: {err}"));
                    }
                }
                Task::none()
            }

            Message::WindowPicked(win) => {
                self.selected_window = Some(win.clone());
                self.config.app_name = win.app_name.clone();
                self.app_name_input = win.app_name;
                self.status = Some("Window selected.".into());
                Task::none()
            }

            Message::AppNameChanged(v) => {
                self.app_name_input = v.clone();
                self.config.app_name = v;
                Task::none()
            }

            Message::PollDelayChanged(v) => {
                self.poll_delay_input = v.clone();
                if let Ok(parsed) = v.trim().parse::<f32>() {
                    self.config.poll_delay_s = parsed.clamp(0.1, 60.0);
                }
                Task::none()
            }

            Message::MaxCaptureHeightChanged(v) => {
                self.max_capture_height_input = v.clone();
                self.config.max_capture_height = v.trim().parse::<u32>().ok();
                Task::none()
            }

            Message::SaveConfig => {
                let cfg = self.config.clone();
                Task::perform(async move { cfg.save().map_err(|e| e.to_string()) }, Message::ConfigSaved)
            }

            Message::ConfigSaved(res) => {
                match res {
                    Ok(_) => self.status = Some("Config saved.".into()),
                    Err(err) => self.status = Some(format!("Config save failed: {err}")),
                }
                Task::none()
            }

            Message::SampleTheme => {
                let app_name = self.config.app_name.clone();
                let max_h = self.config.max_capture_height;
                let ie = self.ie.clone();
                Task::perform(
                    async move {
                        let img = capture_by_app_name(&app_name, max_h).map_err(|e| e.to_string())?;
                        let theme = ie::Theme::from_options(img.as_image());
                        // Update engine theme immediately.
                        if let Ok(mut guard) = ie.lock() {
                            guard.set_theme(theme);
                        }
                        Ok(theme)
                    },
                    Message::ThemeSampled,
                )
            }

            Message::ThemeSampled(res) => {
                match res {
                    Ok(theme) => {
                        self.config.theme = theme;
                        self.status = Some(format!(
                            "Theme sampled: primary=({}, {}, {}), secondary=({}, {}, {})",
                            theme.primary.r,
                            theme.primary.g,
                            theme.primary.b,
                            theme.secondary.r,
                            theme.secondary.g,
                            theme.secondary.b
                        ));
                    }
                    Err(err) => self.status = Some(format!("Theme sample failed: {err}")),
                }
                Task::none()
            }

            Message::PollPartyHeaderNow => {
                if self.poll.party_in_flight {
                    return Task::none();
                }
                self.poll.party_in_flight = true;

                let app_name = self.config.app_name.clone();
                let max_h = self.config.max_capture_height;
                let ie = self.ie.clone();

                Task::perform(
                    async move {
                        let img = capture_by_app_name(&app_name, max_h).map_err(|e| e.to_string())?;
                        let text = ie
                            .lock()
                            .map_err(|_| "IE mutex poisoned".to_string())?
                            .util_party_header_text(&img);
                        Ok(text)
                    },
                    Message::PartyHeaderPolled,
                )
            }

            Message::PartyHeaderPolled(res) => {
                self.poll.party_in_flight = false;

                match res {
                    Ok(Some(text)) => {
                        self.debug.last_party_header = Some(text.clone());
                        self.status = Some(format!("Party header: {text}"));

                        // Enter reward-mode for a short window to auto-refresh rewards.
                        let now = Instant::now();
                        self.poll.reward_mode_until = Some(now + Duration::from_secs(3));
                        self.poll.next_reward_poll = now;
                    }
                    Ok(None) => {
                        self.status = Some("Party header: <none>".into());
                    }
                    Err(err) => {
                        self.status = Some(format!("Party header poll failed: {err}"));
                    }
                }

                Task::none()
            }

            Message::PollRelicNow => {
                if self.poll.reward_in_flight {
                    return Task::none();
                }
                self.poll.reward_in_flight = true;

                let app_name = self.config.app_name.clone();
                let max_h = self.config.max_capture_height;
                let ie = self.ie.clone();

                Task::perform(
                    async move {
                        let img = capture_by_app_name(&app_name, max_h).map_err(|e| e.to_string())?;
                        let guard = ie.lock().map_err(|_| "IE mutex poisoned".to_string())?;
                        let rewards = guard.relicreward_get_rewards(&img);
                        let selected = guard.relicreward_get_selected(&img);
                        Ok(RelicPollResult { rewards, selected })
                    },
                    Message::RelicPolled,
                )
            }

            Message::RelicPolled(res) => {
                self.poll.reward_in_flight = false;

                match res {
                    Ok(v) => {
                        self.relic.rewards = Some(v.rewards);
                        self.relic.selected = v.selected;
                        self.relic.last_updated = Some(Instant::now());
                    }
                    Err(err) => {
                        self.status = Some(format!("Relic poll failed: {err}"));
                    }
                }
                Task::none()
            }

            Message::ValuedFormaToggled(v) => {
                self.relic.valued_forma = v;
                Task::none()
            }
        }
    }

    fn on_tick(&mut self, now: Instant) -> Task<Message> {
        // 1) Lightweight periodic party header poll.
        let delay = Duration::from_secs_f32(self.config.poll_delay_s.max(0.1));
        if !self.poll.party_in_flight && now.duration_since(self.poll.last_party_poll) >= delay {
            self.poll.last_party_poll = now;
            return self.update(Message::PollPartyHeaderNow);
        }

        // 2) While in reward-mode, refresh rewards at a moderate interval.
        if let Some(until) = self.poll.reward_mode_until {
            if now <= until && !self.poll.reward_in_flight && now >= self.poll.next_reward_poll {
                self.poll.next_reward_poll = now + Duration::from_millis(350);
                return self.update(Message::PollRelicNow);
            }

            if now > until {
                self.poll.reward_mode_until = None;
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<Message> {
        let tabs = Row::new()
            .spacing(10)
            .push(tab_button(self.tab, Tab::RelicRewards))
            .push(tab_button(self.tab, Tab::Settings))
            .push(tab_button(self.tab, Tab::Debug));

        let content = match self.tab {
            Tab::RelicRewards => self.view_relic(),
            Tab::Settings => self.view_settings(),
            Tab::Debug => self.view_debug(),
        };

        let mut root = Column::new()
            .spacing(10)
            .padding(10)
            .push(Text::new("WFBuddy").size(28))
            .push(tabs)
            .push(Container::new(content).padding(10).width(Length::Fill).height(Length::Fill));

        if let Some(status) = &self.status {
            root = root.push(Container::new(Text::new(status)).padding(8));
        }

        root.into()
    }

    fn view_settings(&self) -> Element<Message> {
        // Ensure both branches return the same type.
        // We keep the internal widget types flexible by converting to `Element`.
        let window_picker: Element<Message> = if self.windows.is_empty() {
            Column::new()
                .spacing(6)
                .push(Text::new("No windows found (or permission issue)."))
                .push(Button::new(Text::new("Refresh Windows")).on_press(Message::RefreshWindows))
                .into()
        } else {
            let mut row = Row::new().spacing(10);

            row = row.push(
                PickList::new(
                    self.windows.clone(),
                    self.selected_window.clone(),
                    Message::WindowPicked,
                )
                .placeholder("Select a window…"),
            );

            row = row.push(Button::new(Text::new("Refresh")).on_press(Message::RefreshWindows));
            row.into()
        };

        let app_name = TextInput::new("app_name (xcap)", &self.app_name_input)
            .on_input(Message::AppNameChanged)
            .width(Length::Fill);

        let poll_delay = TextInput::new("poll delay (seconds)", &self.poll_delay_input)
            .on_input(Message::PollDelayChanged)
            .width(Length::Fixed(160.0));

        let max_h = TextInput::new("max capture height (blank=off)", &self.max_capture_height_input)
            .on_input(Message::MaxCaptureHeightChanged)
            .width(Length::Fixed(220.0));

        let theme = self.config.theme;
        let theme_text = Text::new(format!(
            "Theme:\n  primary:   ({}, {}, {})\n  secondary: ({}, {}, {})",
            theme.primary.r,
            theme.primary.g,
            theme.primary.b,
            theme.secondary.r,
            theme.secondary.g,
            theme.secondary.b
        ));

        Column::new()
            .spacing(12)
            .push(Text::new("Target Window"))
            .push(window_picker)
            .push(Text::new("Or set by app_name:"))
            .push(app_name)
            .push(Row::new().spacing(10).push(poll_delay).push(max_h))
            .push(Row::new().spacing(10).push(Button::new(Text::new("Save Config")).on_press(Message::SaveConfig)))
            .push(Row::new().spacing(10).push(Button::new(Text::new("Sample Theme (capture)")).on_press(Message::SampleTheme)))
            .push(theme_text)
            .into()
    }

    fn view_relic(&self) -> Element<Message> {
        let mut col = Column::new().spacing(10);

        col = col.push(
            Row::new()
                .spacing(10)
                .push(Button::new(Text::new("Poll Now")).on_press(Message::PollRelicNow))
                .push(
                    // iced 0.14: Checkbox::new only takes the checked state.
                    // Set the label separately.
                    Checkbox::new(self.relic.valued_forma)
                        .label("Valued Forma")
                        .on_toggle(Message::ValuedFormaToggled),
                ),
        );

        if let Some(rewards) = &self.relic.rewards {
            col = col.push(Text::new(format!("Timer: {}s", rewards.timer)));

            let mut list = Column::new().spacing(6);

            for (i, r) in rewards.rewards.iter().enumerate() {
                let selected = self.relic.selected == Some(i);
                let name = if r.name.is_empty() { "<unknown>".to_string() } else { r.name.clone() };

                let (ducats, vaulted, is_relic_item) = self.lookup_item(&name);

                let ducats = if !self.relic.valued_forma && name.to_lowercase().contains("forma") {
                    0
                } else {
                    ducats.unwrap_or(0)
                };

                let mut line = format!(
                    "#{:02}  owned:{:<2}  ducats:{:<3}  {}",
                    i + 1,
                    r.owned,
                    ducats,
                    name
                );

                if vaulted.unwrap_or(false) {
                    line.push_str("  [VAULTED]");
                }
                if is_relic_item.unwrap_or(false) {
                    line.push_str("  [RELIC]");
                }

				let text = Text::new(line);

				// Highlight the currently-selected reward row.
				let style = if selected { container::primary } else { container::transparent };
				list = list.push(Container::new(text).padding(6).style(style));
            }

            col = col.push(Scrollable::new(list).height(Length::Fill));
        } else {
            col = col
                .push(Text::new("No rewards yet."))
                .push(Text::new("Tip: Make sure the target window is selected, then click 'Poll Now' while on the relic reward screen."));
        }

        col.into()
    }

    fn view_debug(&self) -> Element<Message> {
        let last = self
            .debug
            .last_party_header
            .clone()
            .unwrap_or_else(|| "<none>".to_string());

        Column::new()
            .spacing(12)
            .push(Button::new(Text::new("Capture + OCR Party Header")).on_press(Message::PollPartyHeaderNow))
            .push(Text::new(format!("Last party header: {last}")))
            .into()
    }

    fn lookup_item(&self, name: &str) -> (Option<u32>, Option<bool>, Option<bool>) {
        let Some(data) = &self.data else {
            return (None, None, None);
        };

        // Best-effort mapping: if anything is missing, return partial info.
        // `data::Data::find_item_name` expects a `(Language, &str)` tuple (or similar)
        // so it can apply language-specific matching.
        let canonical = data.find_item_name((data::Language::English, name));

        let id = data.id_manager.get_id_from_en(canonical);
        let Some(id) = id else {
            return (None, None, None);
        };

        let ducats = data.ducat_values.get(&id).copied();
        let vaulted = Some(data.vaulted_items.contains(&id));
        let relic = Some(data.relic_items.contains(&id));

        (ducats, vaulted, relic)
    }
}

fn tab_button(current: Tab, tab: Tab) -> Element<'static, Message> {
    let label = Text::new(tab.to_string());
    let style = if current == tab { button::primary } else { button::secondary };
	Button::new(label)
		.on_press(Message::TabSelected(tab))
		.style(style)
		.into()
}

fn resolve_model_path(rel: &str) -> PathBuf {
    let rel = PathBuf::from(rel);

    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join(&rel);
        if p.exists() {
            return p;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(&rel);
            if p.exists() {
                return p;
            }
        }
    }

    rel
}
