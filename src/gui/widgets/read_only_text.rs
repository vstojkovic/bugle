use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use fltk::enums::Align;
use fltk::prelude::*;
use fltk::text::{Cursor, TextBuffer, TextEditor};
use fltk_float::text::TextElement;
use fltk_float::{IntoWidget, LayoutElement, LayoutWidgetWrapper};

#[derive(Clone)]
pub struct ReadOnlyText {
    editor: TextEditor,
    value: Rc<RefCell<String>>,
}

impl ReadOnlyText {
    pub fn new(initial_value: String) -> Self {
        let mut buffer = TextBuffer::default();
        buffer.set_text(&initial_value);

        let mut editor = TextEditor::default();
        editor.set_buffer(buffer.clone());
        editor.show_cursor(true);
        editor.set_cursor_style(Cursor::Simple);
        editor.set_scrollbar_align(Align::Clip);

        let value = Rc::new(RefCell::new(initial_value));
        {
            let mut editor = editor.clone();
            let mut buffer = buffer.clone();
            let value = Rc::clone(&value);
            buffer
                .clone()
                .add_modify_callback(move |pos, ins, del, _, _| {
                    if (ins > 0) || (del > 0) {
                        if let Ok(value) = value.try_borrow_mut() {
                            buffer.set_text(&value);
                            editor.set_insert_position(pos);
                        }
                    }
                });
        }

        Self { editor, value }
    }

    pub fn set_value(&self, value: String) {
        let mut value_ref = self.value.borrow_mut();
        self.editor.buffer().unwrap().set_text(&value);
        *value_ref = value;
    }
}

impl Default for ReadOnlyText {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Deref for ReadOnlyText {
    type Target = TextEditor;
    fn deref(&self) -> &Self::Target {
        &self.editor
    }
}

impl DerefMut for ReadOnlyText {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.editor
    }
}

impl IntoWidget for ReadOnlyText {
    fn into_widget(self) -> fltk::widget::Widget {
        self.editor.as_base_widget()
    }
}

pub struct ReadOnlyTextElement {
    inner: TextElement<TextEditor>,
}

impl LayoutWidgetWrapper<ReadOnlyText> for ReadOnlyTextElement {
    fn wrap(widget: ReadOnlyText) -> Self {
        Self {
            inner: TextElement::wrap(widget.editor),
        }
    }
}

impl LayoutElement for ReadOnlyTextElement {
    fn min_size(&self) -> fltk_float::Size {
        self.inner.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.inner.layout(x, y, width, height);
    }
}
