//! Making a player card: a name, an animal, a colour and a grade.

use aizome_kit::layout::Point;
use aizome_kit::prelude::*;
use aizome_kit::sidestep_ui::kurbo::Circle;
use aizome_kit::sidestep_ui::{Key, NamedKey, StrokeStyle};
use aizome_kit::widget::Event;
use aizome_kit::{column, row};
use sekka_kid::{Animal, Grade, KidColor, NewProfile};

use crate::avatar::{self, Mood, kid_color};
use crate::look::Look;

/// The form as filled in so far.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NewKid {
    pub name: String,
    pub animal: Animal,
    pub color: KidColor,
    pub grade: Grade,
}

impl NewKid {
    /// A fresh form; the `n`th kid gets a different animal and colour to
    /// start from than the kids before.
    pub fn new(n: usize) -> NewKid {
        NewKid {
            name: String::new(),
            animal: Animal::ALL[(n * 3 + 1) % Animal::ALL.len()],
            color: KidColor::ALL[(n * 2 + 1) % KidColor::ALL.len()],
            grade: Grade::Kindergarten,
        }
    }

    pub fn ready(&self) -> bool {
        !NewProfile::clean_name(&self.name).is_empty()
    }

    pub fn profile(&self) -> NewProfile {
        NewProfile { name: self.name.clone(), animal: self.animal, color: self.color, grade: self.grade }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewKidMsg {
    Name(String),
    Animal(Animal),
    Color(KidColor),
    Grade(Grade),
    Make,
    Cancel,
}

fn heading(s: &str, size: f64, look: &Look) -> Element<NewKidMsg> {
    rich_text([Span::new(s).color(look.ink).size(size).weight(800.0)]).into()
}

fn activated(e: &Event) -> bool {
    match e {
        Event::PointerUp { .. } => true,
        Event::Key(k) if k.pressed => {
            matches!(&k.key, Key::Named(NamedKey::Enter)) || k.key == Key::Character(" ".into())
        }
        _ => false,
    }
}

/// An animal to pick.
fn animal_choice(animal: Animal, color: KidColor, chosen: bool) -> Element<NewKidMsg> {
    canvas((animal, color, chosen), |c, b, (animal, color, chosen), theme| {
        let look = Look::for_theme(theme);
        let at = b.center();
        if *chosen {
            c.fill(&Circle::new(at, b.width() / 2.0 - 2.0), kid_color(*color).mix(look.card, 0.35));
            c.stroke(&Circle::new(at, b.width() / 2.0 - 2.0), &StrokeStyle::new(3.0), kid_color(*color));
        }
        avatar::paint(c, Point::new(at.x, at.y + 4.0), b.width() * 0.27, *animal, None, Mood::Happy);
    })
    .on_event(move |e, _, _| activated(e).then_some(NewKidMsg::Animal(animal)))
    .focusable()
    .label(animal.name())
    .width(64.0)
    .height(64.0)
    .into()
}

/// A colour to pick.
fn color_choice(color: KidColor, chosen: bool) -> Element<NewKidMsg> {
    canvas((color, chosen), |c, b, (color, chosen), theme| {
        let look = Look::for_theme(theme);
        let at = b.center();
        c.fill(&Circle::new(at, 17.0), kid_color(*color));
        if *chosen {
            c.stroke(&Circle::new(at, 22.0), &StrokeStyle::new(3.0), look.ink);
        }
    })
    .on_event(move |e, _, _| activated(e).then_some(NewKidMsg::Color(color)))
    .focusable()
    .label(color.name())
    .width(48.0)
    .height(48.0)
    .into()
}

/// The form, on a card in the kids' look.
pub fn view(form: &NewKid, can_cancel: bool, look: &Look) -> Element<NewKidMsg> {
    let animals: Vec<Element<NewKidMsg>> =
        Animal::ALL.iter().map(|a| animal_choice(*a, form.color, *a == form.animal)).collect();
    let colors: Vec<Element<NewKidMsg>> = KidColor::ALL.iter().map(|k| color_choice(*k, *k == form.color)).collect();
    let grade = Grade::ALL.iter().position(|g| *g == form.grade).unwrap_or(1);
    let preview = canvas(form.clone(), |c, b, form, _| {
        avatar::paint(c, b.center(), b.height() * 0.36, form.animal, Some(form.color), Mood::Cheer);
    })
    .width(150.0)
    .height(150.0);
    let mut buttons: Vec<Element<NewKidMsg>> = vec![spacer().into()];
    if can_cancel {
        buttons.push(button("Not now").cancel().on_press(NewKidMsg::Cancel).into());
    }
    buttons.push(button("Make my card").default().enabled(form.ready()).on_press(NewKidMsg::Make).into());
    let form_col = column![
        heading("New player", 30.0, look),
        text_field(&form.name).placeholder("Name").on_change(NewKidMsg::Name).on_submit(NewKidMsg::Make).id("name"),
        heading("Pick an animal", 18.0, look),
        aizome_kit::widgets::layout::row(animals).spacing(4.0),
        heading("Pick a colour", 18.0, look),
        aizome_kit::widgets::layout::row(colors).spacing(6.0),
        heading("Grade", 18.0, look),
        segmented(Grade::ALL.map(|g| g.short()), grade).on_select(|i| NewKidMsg::Grade(Grade::ALL[i])),
        space(6.0),
        aizome_kit::widgets::layout::row(buttons).spacing(10.0),
    ]
    .spacing(12.0)
    .width(560.0);
    let card = container(row![preview, form_col].spacing(20.0).align(Align::Start))
        .padding(28.0)
        .fill(Tone::Color(look.card))
        .floating();
    container(card).center().width(Length::Fill).height(Length::Fill).into()
}
