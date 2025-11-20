use tera::{Tera, Context};
use crate::Note;

impl Note {
    /// Render note with given template to HTML string
    pub fn render_html(&self, template: &str) -> String {
        let mut tera = Tera::new(template).unwrap();
        // disable escape for HTML templates
        tera.autoescape_on(vec![]);

        let ctx = Context::from_serialize(&self).unwrap();

        tera.render("template.html", &ctx).unwrap()
    }
}
