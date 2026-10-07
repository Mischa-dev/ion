//! Page controls in Ion's colors (`theme.pageControls`): scrollbars, form
//! controls such as checkboxes and sliders, and text selection.
//!
//! The CSS sits in a cascade layer, so any style of the page's own wins over
//! it: only pages that leave these controls at the browser's defaults change.

use crate::{Color, Palette};

/// The cascade layer the CSS goes in.
const LAYER: &str = "ion-page-controls";

/// CSS that gives page controls `palette`'s colors.
pub fn css(palette: &Palette) -> String {
    let thumb = alpha(palette.text_muted, 0x99);
    let selection = alpha(palette.accent, 0x59);
    let accent = palette.accent;
    format!(
        "@layer {LAYER} {{\n\
         :root {{ accent-color: {accent}; scrollbar-color: {thumb} transparent; }}\n\
         ::selection {{ background-color: {selection}; }}\n\
         }}\n"
    )
}

fn alpha(c: Color, a: u8) -> Color {
    Color::rgba(c.r, c.g, c.b, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_palette_in_a_layer() {
        let palette = crate::builtin::get(crate::builtin::DEFAULT_DARK).unwrap();
        let css = css(&palette);
        assert!(css.starts_with("@layer ion-page-controls {"), "{css}");
        assert!(css.contains(&format!("accent-color: {};", palette.accent)));
        assert!(css.contains(&format!(
            "scrollbar-color: {} transparent;",
            alpha(palette.text_muted, 0x99)
        )));
        assert!(css.contains(&format!(
            "::selection {{ background-color: {}",
            alpha(palette.accent, 0x59)
        )));
    }
}
