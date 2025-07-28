use std::sync::LazyLock;

use dsb::{Fonts, cell::Style};
use swash::FontRef;

fn main() {
    let ppem = 15.0;
    let lh = 200.0;
    // let (fw, fh) = dsb::dims(&FONT, ppem);
    let (w, h) = (1920, 1080);
    dbg!(w, h);
    let (c, r) = dsb::fit(&FONT, ppem, lh, (w, h));

    dbg!(c, r);
    // panic!();
    let cells = include_str!("../src/lib.rs")
        .chars()
        .filter(|x| !x.is_whitespace())
        .take((c * r) as _)
        .map(|x: char| dsb::Cell {
            style: Style {
                bg: [0; 3],
                color: [255; 3],
                flags: 0,
            },
            letter: Some(x),
        })
        .collect::<Vec<_>>();
    let x = unsafe {
        dsb::render(
            &cells,
            (c, r),
            ppem,
            [1; 3],
            Fonts::new(*FONT, *FONT, *FONT, *FONT),
            lh,
        )
    };
    x.save("yes.png");
    assert!(x.height() < h as u32);
    dbg!(x.width(), x.height());
}
pub static FONT: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::from_index(&include_bytes!("/home/os/CascadiaCodeNF.ttf")[..], 0).unwrap()
});
