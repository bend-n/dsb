use std::sync::LazyLock;

use dsb::{Fonts, cell::Style};
use swash::FontRef;

fn main() {
    let ppem = 300.0;
    let lh = 0.0;
    let (fw, fh) = dsb::dims(&FONT, ppem, lh);
    let (w, h) = (2000, 2000);

    let cols = (w as f32 / fw).floor() as usize;
    let rows = (h as f32 / fh).floor() as usize;

    dbg!(cols, rows);

    let cells = //include_str!("../src/lib.rs")
    "||||ppppppre|rlly rea(ly regl(y fugck thgis i plight is hard plifhgt plongkokignpookfiokogk"
        .chars()
        .filter(|x| !x.is_whitespace())
        .take(cols * rows)
        .map(|x: char| dsb::Cell {
            style: Style {
                bg: [0; 3],
                color: [255; 3],
                flags: 0,
            },
            letter: Some(x),
        })
        .collect::<Vec<_>>();
    unsafe {
        dsb::render(
            &cells,
            (cols, rows),
            ppem,
            [1; 3],
            Fonts::new(*FONT, *FONT, *FONT, *FONT),
            lh,
        )
    }
    .show();
}
pub static FONT: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::from_index(&include_bytes!("/home/os/CascadiaCodeNF.ttf")[..], 0).unwrap()
});
