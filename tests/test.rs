use std::sync::LazyLock;
use std::time::Instant;

use dsb::Fonts;
use dsb::cell::Style;
use fimg::Image;
use swash::FontRef;

fn main() {
    let ppem = 15.0;
    let lh = 200.0;
    // let (fw, fh) = dsb::dims(&FONT, ppem);
    let (w, h) = (2160, 1440);
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
                bg: [255; 3],
                color: [132, 148, 164],
                flags: 0,
            },
            letter: Some(x),
        })
        .collect::<Vec<_>>();
    let now = Instant::now();
    let mut x = Image::alloc(w as _, h as _);
    unsafe {
        dsb::render(
            &cells,
            (c, r),
            ppem,
            [255; 3],
            &mut Fonts::new(*FONT, *FONT, *FONT, *FONT),
            lh,
            true,
            x.as_mut(),
        )
    };
    println!("{:?}", now.elapsed());
    x.as_ref().show();
}
pub static FONT: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::from_index(
        &include_bytes!("/home/os/CascadiaCodeNF.ttf")[..],
        0,
    )
    .unwrap()
});
