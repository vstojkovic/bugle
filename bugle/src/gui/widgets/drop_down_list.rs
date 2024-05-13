use std::cell::Cell;
use std::rc::Rc;

use fltk::button::Button;
use fltk::enums::{Align, Color, FrameType};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::menu::MenuButton;
use fltk::prelude::*;
use fltk_float::{IntoWidget, LayoutElement, LayoutWidgetWrapper, Size};

use crate::gui::prelude::WidgetConvenienceExt;

// DropDownList is like Choice, but it doesn't try to position the currently selected menu item
// in line with the text portion of the widget

#[derive(Clone)]
pub struct DropDownList {
    group: Group,
    text: Frame,
    menu: MenuButton,
    button: Button,
    value: Rc<Cell<i32>>,
}

// Explanation for posterity: why do we need a Button and a MenuButton? In FLTK, menus are arrays
// of Fl_Menu_Item structs, dynamically allocated. In fltk-rs, MenuItem is currently an Arc or a Rc
// (depending on configuration) to a wrapper around a *mut Fl_Menu_Item, and there is no way to
// dynamically allocate Fl_Menu_Item array without leaking memory. FLTK menu widgets manage menu
// items properly, so we use a hidden MenuButton to hold menu items, for now.

pub struct DropDownListElement {
    widget: DropDownList,
}

impl DropDownList {
    pub fn default_fill() -> Self {
        let group = Group::default_fill();
        let text = Frame::default_fill();
        let button = Button::default();
        let menu = MenuButton::default();
        group.end();
        Self {
            group,
            text,
            button,
            menu,
            value: Rc::new(Cell::new(-1)),
        }
        .init_children()
    }

    pub fn add(&mut self, option: &str) {
        self.menu.add_choice(option);
    }

    pub fn set_activated(&mut self, activated: bool) {
        self.text.set_activated(activated);
        self.button.set_activated(activated);
    }

    pub fn choice(&self) -> Option<String> {
        if self.value() >= 0 {
            self.menu.choice()
        } else {
            None
        }
    }

    pub fn value(&self) -> i32 {
        self.value.get()
    }

    pub fn set_value<V: Into<i32>>(&self, value: V) {
        let value = value.into();
        self.value.set(value);
        if value >= 0 {
            self.menu.clone().set_value(value);
        }
        self.text
            .clone()
            .set_label(&self.choice().unwrap_or_default());
        self.button.clone().redraw();
    }

    pub fn set_callback<F: FnMut(&mut Self) + 'static>(&mut self, mut cb: F) {
        self.button.set_callback({
            let mut this = self.clone();
            move |_| {
                let menu = this.menu.menu().unwrap();
                let picked = menu.pulldown(
                    this.group.x(),
                    this.group.y(),
                    this.group.w(),
                    this.group.h(),
                    None,
                    Some(&this.menu),
                );
                if picked.is_some() {
                    this.menu.set_item(&picked.unwrap());
                    this.value.set(this.menu.value());
                }
                let text = this.choice().unwrap_or_default();
                this.text.set_label(&text);
                this.button.redraw();
                cb(&mut this);
            }
        });
    }

    fn init_children(mut self) -> Self {
        self.text.set_align(Align::Left | Align::Inside);
        self.text.set_frame(FrameType::DownBox);
        self.text.set_color(Color::Background2);
        self.text.set_label_color(Color::Foreground);

        self.button.set_label("@#-12>");

        self.menu.hide();

        self.on_resize(
            self.group.x(),
            self.group.y(),
            self.group.w(),
            self.group.h(),
        );
        self.set_callback(|_| ());

        self.group.resize_callback({
            let mut this = self.clone();
            move |_, x, y, w, h| this.on_resize(x, y, w, h)
        });

        self
    }

    fn on_resize(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.text.resize(x, y, w, h);

        let frame = self.text.frame();
        let button_w = std::cmp::max(0, std::cmp::min(w - frame.dw(), BUTTON_WIDTH));
        self.button.clone().resize(
            x + w - frame.dw() + frame.dx() - button_w,
            y + frame.dy(),
            button_w,
            h - frame.dh(),
        );
    }
}

impl Default for DropDownList {
    fn default() -> Self {
        let group = Group::default();
        let text = Frame::default();
        let button = Button::default();
        let menu = MenuButton::default();
        group.end();
        Self {
            group,
            text,
            button,
            menu,
            value: Rc::new(Cell::new(-1)),
        }
        .init_children()
    }
}

impl WidgetConvenienceExt for DropDownList {
    fn set_activated(&mut self, activated: bool) {
        self.group.set_activated(activated);
    }

    fn with_tooltip(self, tooltip: &str) -> Self {
        self.group.clone().set_tooltip(tooltip);
        self
    }
}

impl IntoWidget for DropDownList {
    fn into_widget(self) -> fltk::widget::Widget {
        self.group.as_base_widget()
    }
}

impl LayoutWidgetWrapper<DropDownList> for DropDownListElement {
    fn wrap(widget: DropDownList) -> Self {
        DropDownListElement { widget }
    }
}

impl LayoutElement for DropDownListElement {
    fn min_size(&self) -> Size {
        let frame = self.widget.text.frame();
        let frame_dx = frame.dx();
        let frame_dy = frame.dy();
        let frame_dw = frame.dw();
        let frame_dh = frame.dh();
        let frame_w = frame_dx + frame_dw;
        let frame_h = frame_dy + frame_dh;

        let widest_option = (0..self.widget.menu.size())
            .into_iter()
            .map(|idx| self.widget.menu.text(idx).unwrap_or_default())
            .max_by_key(|s| s.len())
            .unwrap_or_default();
        fltk::draw::set_font(self.widget.text.label_font(), self.widget.text.label_size());
        let (label_w, label_h) = fltk::draw::measure(&widest_option, true);
        let text_w = label_w + 2 * frame_w;
        let text_h = label_h + frame_h;

        Size {
            width: text_w + BUTTON_WIDTH,
            height: text_h,
        }
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.widget.group.clone().resize(x, y, width, height);
    }
}

const BUTTON_WIDTH: i32 = 20; // from FLTK source
