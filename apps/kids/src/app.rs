//! Sekka Kids: the grown-ups' place for kids' profiles. Each kid's report
//! across the Sekka Edu catalogue (time, rounds, how each skill is going),
//! their card, their daily time, and removing them. Changes go through the
//! grown-ups' door; reading the reports doesn't.

use std::os::unix::net::UnixListener;
use std::rc::Rc;
use std::time::Duration;

use aizome_kit::prelude::*;
use aizome_kit::{column, row};
use sekka_kid::{Animal, Grade, KidColor, Profile, Report, Store, clock, report};
use sekka_kid_kit::gate::{self, Gate, GateMsg};
use sekka_kid_kit::newkid::{self, NewKid, NewKidMsg};
use sekka_kid_kit::{Look, instance, report_view};
use sekka_voice::{Voice, hints};

pub const APP_ID: &str = "computer.sekka.Kids";

/// How often the reports are read again (apps write the logs as kids play).
const REFRESH: Duration = Duration::from_secs(30);

/// The periods a report can cover.
pub const PERIODS: [(usize, &str); 3] = [(7, "Week"), (14, "Two weeks"), (30, "Month")];

#[derive(Clone, Debug)]
pub enum Msg {
    Select(usize),
    Period(usize),
    Refresh,
    New,
    Edit,
    Remove,
    Removed,
    Gate(GateMsg),
    Form(FormMsg),
    NewKid(NewKidMsg),
    Raise,
}

/// The card and the day's allowance, being changed.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub name: String,
    /// The name spelt how it sounds, for the apps' voices.
    pub sounds_like: String,
    pub animal: Animal,
    pub color: KidColor,
    pub grade: Grade,
    pub limit: bool,
    pub minutes: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FormMsg {
    Name(String),
    SoundsLike(String),
    /// Say hello to the kid, as the apps will.
    Hear,
    Animal(usize),
    Color(usize),
    Grade(usize),
    Limit(bool),
    Minutes(f64),
    Save,
    Cancel,
}

/// What the door is guarding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Guarded {
    New,
    Edit,
    Remove,
}

#[derive(Clone, Debug)]
enum Sheet {
    Gate(Gate, Guarded),
    Form(Form),
    New(NewKid),
}

pub struct Kids {
    pub store: Store,
    pub clock: Rc<dyn Fn() -> i64>,
    pub instance: Option<UnixListener>,
    window: WindowKey,
    sheet: Option<(WindowKey, Sheet)>,
    pub kids: Vec<Profile>,
    pub selected: usize,
    pub period: usize,
    pub report: Option<Report>,
    /// The door was opened once: grown-ups needn't answer again while the
    /// window is open.
    unlocked: bool,
    seed: u64,
    /// Night (the theme the view can't ask for).
    dark: bool,
    /// The apps' voice, to hear a name: loaded the first time it's asked
    /// for.
    voice: Option<Voice>,
    /// Never make a sound (tests).
    pub quiet: bool,
}

impl Kids {
    pub fn new(store: Store) -> Kids {
        Kids {
            store,
            clock: Rc::new(clock::now_ms),
            instance: None,
            window: WindowKey::NONE,
            sheet: None,
            kids: Vec::new(),
            selected: 0,
            period: 0,
            report: None,
            unlocked: false,
            seed: clock::now_ms() as u64,
            dark: true,
            voice: None,
            quiet: false,
        }
    }

    fn kid(&self) -> Option<&Profile> {
        self.kids.get(self.selected)
    }

    /// Read the kids and the selected kid's report again.
    pub fn refresh(&mut self) {
        let id = self.kid().map(|k| k.id.clone());
        self.kids = self.store.profiles();
        self.selected = id.and_then(|id| self.kids.iter().position(|k| k.id == id)).unwrap_or(0);
        let now = (self.clock)();
        self.report = self.kid().map(|k| report(&self.store, k, now, PERIODS[self.period].0));
    }

    fn sheet(&mut self, s: Sheet, cx: &mut Cx<Msg>) {
        match self.sheet.as_mut() {
            Some((_, old)) => *old = s,
            None => {
                let key = cx.show_sheet(self.window);
                self.sheet = Some((key, s));
            }
        }
    }

    fn close_sheet(&mut self, cx: &mut Cx<Msg>) {
        if let Some((k, _)) = self.sheet.take() {
            cx.close_window(k);
        }
    }

    /// Do `what`, through the door if it's still shut.
    fn guarded(&mut self, what: Guarded, cx: &mut Cx<Msg>) {
        if !self.unlocked {
            self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            self.sheet(Sheet::Gate(Gate::new(self.seed >> 33), what), cx);
            cx.focus("gate-answer");
            return;
        }
        match what {
            Guarded::New => {
                self.sheet(Sheet::New(NewKid::new(self.kids.len())), cx);
                cx.focus("name");
            }
            Guarded::Edit => {
                let Some(k) = self.kid() else { return };
                let form = Form {
                    name: k.name.clone(),
                    sounds_like: k.sounds_like.clone().unwrap_or_default(),
                    animal: k.animal,
                    color: k.color,
                    grade: k.grade,
                    limit: k.daily_minutes.is_some(),
                    minutes: k.daily_minutes.unwrap_or(30),
                };
                self.sheet(Sheet::Form(form), cx);
            }
            Guarded::Remove => {
                self.close_sheet(cx);
                let Some(k) = self.kid() else { return };
                cx.alert(
                    self.window,
                    Alert::new(
                        format!("Remove {}?", k.name),
                        "Their card goes, and with it what they've played in every Sekka Edu app.",
                    )
                    .destructive_button("Remove", Msg::Removed)
                    .cancel_button("Cancel"),
                );
            }
        }
    }

    fn form(&mut self, m: FormMsg, cx: &mut Cx<Msg>) {
        let Some((_, Sheet::Form(f))) = self.sheet.as_mut() else { return };
        match m {
            FormMsg::Name(n) => f.name = n,
            FormMsg::SoundsLike(s) => f.sounds_like = s,
            FormMsg::Hear => {
                let line = format!("Hi, {}!", hints::hint(f.name.trim(), &f.sounds_like));
                let quiet = self.quiet;
                self.voice.get_or_insert_with(|| Voice::new(true, quiet)).say(line);
            }
            FormMsg::Animal(i) => f.animal = Animal::ALL[i.min(Animal::ALL.len() - 1)],
            FormMsg::Color(i) => f.color = KidColor::ALL[i.min(KidColor::ALL.len() - 1)],
            FormMsg::Grade(i) => f.grade = Grade::ALL[i.min(Grade::ALL.len() - 1)],
            FormMsg::Limit(on) => f.limit = on,
            FormMsg::Minutes(m) => f.minutes = m.round().clamp(5.0, 240.0) as u32,
            FormMsg::Cancel => self.close_sheet(cx),
            FormMsg::Save => {
                let f = f.clone();
                if let Some(k) = self.kid().cloned() {
                    let p = Profile {
                        name: f.name.clone(),
                        sounds_like: sekka_kid::NewProfile::clean_sound(Some(&f.sounds_like)),
                        animal: f.animal,
                        color: f.color,
                        grade: f.grade,
                        daily_minutes: f.limit.then_some(f.minutes),
                        ..k
                    };
                    if sekka_kid::NewProfile::clean_name(&p.name).is_empty() {
                        return;
                    }
                    if let Err(e) = self.store.update(&p) {
                        log::warn!("{}: {e}", self.store.root().display());
                    }
                }
                self.close_sheet(cx);
                self.refresh();
            }
        }
    }

    fn form_view(f: &Form) -> Element<Msg> {
        let m = Msg::Form;
        column![
            text("Edit card").size(20.0).bold(),
            form_section(
                "Card",
                vec![
                    form_row("Name", text_field(&f.name).on_change(move |n| m(FormMsg::Name(n))).width(220.0)).into(),
                    form_row(
                        "Say it as",
                        row![
                            text_field(&f.sounds_like)
                                .placeholder(f.name.trim())
                                .on_change(move |s| m(FormMsg::SoundsLike(s)))
                                .width(150.0),
                            button("Hear it").on_press(m(FormMsg::Hear)),
                        ]
                        .spacing(8.0)
                    )
                    .description(
                        "Their name spelt how it sounds, for Pon and Fumi: “Ree-ah” for Ria. Left empty, they say it as it's written."
                    )
                    .into(),
                    form_row(
                        "Animal",
                        dropdown(Animal::ALL.map(|a| a.name()), Animal::ALL.iter().position(|a| *a == f.animal))
                            .on_select(move |i| m(FormMsg::Animal(i)))
                    )
                    .into(),
                    form_row(
                        "Colour",
                        dropdown(KidColor::ALL.map(|c| c.name()), KidColor::ALL.iter().position(|c| *c == f.color))
                            .on_select(move |i| m(FormMsg::Color(i)))
                    )
                    .into(),
                    form_row(
                        "Grade",
                        dropdown(Grade::ALL.map(|g| g.name()), Grade::ALL.iter().position(|g| *g == f.grade))
                            .on_select(move |i| m(FormMsg::Grade(i)))
                    )
                    .into(),
                ]
            ),
            form_section(
                "Daily time",
                vec![
                    form_row("Limit play each day", toggle(f.limit).on_toggle(move |on| m(FormMsg::Limit(on))))
                        .description("Across every Sekka Edu app. Apps finish the round being played, then rest.")
                        .into(),
                    form_row(
                        "Minutes a day",
                        stepper(f64::from(f.minutes))
                            .range(5.0..=240.0)
                            .step(5.0)
                            .suffix(" min")
                            .on_change(move |v| m(FormMsg::Minutes(v)))
                    )
                    .into(),
                ]
            ),
            row![
                spacer(),
                button("Cancel").cancel().on_press(m(FormMsg::Cancel)),
                button("Save").default().on_press(m(FormMsg::Save)),
            ]
            .spacing(10.0),
        ]
        .spacing(14.0)
        .padding(24.0)
        .width(480.0)
        .into()
    }
}

impl Program for Kids {
    type Msg = Msg;

    fn info(&self) -> AppInfo {
        let icon = Icon::from_name("users").unwrap_or(Icon::BookOpen);
        AppInfo::new(APP_ID, "Sekka Kids", "Kids' Profiles").icon(icon).version(env!("CARGO_PKG_VERSION"))
    }

    fn launched(&mut self, cx: &mut Cx<Msg>) {
        self.window = cx.open_window(WindowSpec::new("Sekka Kids").size(980.0, 700.0).min_size(720.0, 480.0));
        self.dark = cx.theme().is_dark();
        if let Err(e) = self.store.register_app(&sekka_kid::AppEntry {
            id: APP_ID.into(),
            name: "Sekka Kids".into(),
            blurb: "Kids' profiles and reports".into(),
        }) {
            log::warn!("{}: {e}", self.store.root().display());
        }
        self.refresh();
        cx.every(REFRESH, Msg::Refresh);
        if let Some(listener) = self.instance.take() {
            instance::serve(listener, cx.stream(), || Msg::Raise);
        }
    }

    fn update(&mut self, msg: Msg, cx: &mut Cx<Msg>) {
        self.dark = cx.theme().is_dark();
        match msg {
            Msg::Select(i) => {
                self.selected = i;
                self.refresh();
            }
            Msg::Period(i) => {
                self.period = i.min(PERIODS.len() - 1);
                self.refresh();
            }
            Msg::Refresh => self.refresh(),
            Msg::New => self.guarded(Guarded::New, cx),
            Msg::Edit => self.guarded(Guarded::Edit, cx),
            Msg::Remove => self.guarded(Guarded::Remove, cx),
            Msg::Removed => {
                if let Some(k) = self.kid().cloned()
                    && let Err(e) = self.store.remove(&k.id)
                {
                    log::warn!("{}: {e}", self.store.root().display());
                }
                self.selected = 0;
                self.refresh();
            }
            Msg::Gate(m) => {
                let Some((_, Sheet::Gate(g, what))) = self.sheet.as_mut() else { return };
                let what = *what;
                match m {
                    GateMsg::Answer(a) => {
                        g.answer = a;
                        g.wrong = false;
                    }
                    GateMsg::Submit if g.right() => {
                        self.unlocked = true;
                        self.guarded(what, cx);
                    }
                    GateMsg::Submit => {
                        g.wrong = true;
                        g.answer.clear();
                    }
                    GateMsg::Cancel => self.close_sheet(cx),
                }
            }
            Msg::Form(m) => self.form(m, cx),
            Msg::NewKid(m) => {
                let Some((_, Sheet::New(form))) = self.sheet.as_mut() else { return };
                match m {
                    NewKidMsg::Name(n) => form.name = n,
                    NewKidMsg::Animal(a) => form.animal = a,
                    NewKidMsg::Color(c) => form.color = c,
                    NewKidMsg::Grade(g) => form.grade = g,
                    NewKidMsg::Cancel => self.close_sheet(cx),
                    NewKidMsg::Make if form.ready() => {
                        let p = form.profile();
                        match self.store.add(p) {
                            Ok(p) => {
                                self.close_sheet(cx);
                                self.refresh();
                                self.selected = self.kids.iter().position(|k| k.id == p.id).unwrap_or(0);
                                self.refresh();
                            }
                            Err(e) => log::warn!("{}: {e}", self.store.root().display()),
                        }
                    }
                    NewKidMsg::Make => {}
                }
            }
            Msg::Raise => cx.activate(self.window),
        }
    }

    fn view(&self, window: WindowKey) -> Element<Msg> {
        if let Some((key, sheet)) = &self.sheet
            && *key == window
        {
            return match sheet {
                Sheet::Gate(g, _) => {
                    gate::view(g, "Changing a kid's card or daily time is for grown-ups.").map(Msg::Gate)
                }
                Sheet::Form(f) => Kids::form_view(f),
                Sheet::New(n) => {
                    newkid::view(n, true, if self.dark { &Look::NIGHT } else { &Look::DAY }).map(Msg::NewKid)
                }
            };
        }
        let user = Icon::from_name("user-round");
        let mut side: Vec<Element<Msg>> = vec![caption("Kids")];
        for (i, k) in self.kids.iter().enumerate() {
            side.push(
                sidebar_item(user, k.name.clone(), i == self.selected)
                    .detail(k.grade.short())
                    .on_press(Msg::Select(i))
                    .into(),
            );
        }
        side.push(space(8.0));
        side.push(button("New kid…").on_press(Msg::New).into());
        let apps = self.store.apps();
        if !apps.is_empty() {
            side.push(space(12.0));
            side.push(caption("Sekka Edu"));
            for a in apps.into_iter().filter(|a| a.id != APP_ID) {
                side.push(
                    column![text(a.name).semibold(), text(a.blurb).role(Role::Text3).size(12.0)].spacing(1.0).into(),
                );
            }
        }
        let sidebar = aizome_kit::widgets::layout::column(side).spacing(4.0).padding(12.0);
        let content: Element<Msg> = match &self.report {
            Some(r) => column![
                toolbar(vec![
                    segmented(PERIODS.map(|p| p.1), self.period).on_select(Msg::Period).into(),
                    spacer().into(),
                    button("Edit card and time…").toolbar().on_press(Msg::Edit).into(),
                    button("Remove…").toolbar().on_press(Msg::Remove).into(),
                ]),
                report_view::view(r),
            ]
            .into(),
            None => container(
                column![
                    text("No kids yet").size(20.0).bold(),
                    text("Add a kid here, or in any Sekka Edu app: each kid's time and progress are kept together.")
                        .role(Role::Text2)
                        .wrap(),
                    button("New kid…").default().on_press(Msg::New),
                ]
                .spacing(12.0)
                .width(360.0),
            )
            .center()
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        };
        sidebar_layout(sidebar, content).into()
    }
}
