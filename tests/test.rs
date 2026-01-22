use std::sync::LazyLock;
use std::time::Instant;

use dsb::Fonts;
use dsb::cell::Style;
use fimg::Image;
use swash::FontRef;

fn main() {
    let ppem = 20.0;
    let lh = -400.0;
    // let (fw, fh) = dsb::dims(&FONT, ppem);
    let (w, h) = (2560, 1440);
    dbg!(w, h);
    let (c, r) = dsb::fit(&FONT, ppem, lh, (w, h));

    dbg!(c, r);
    // panic!();
    let cells = include_str!("haus.txt")
        .chars()
        .filter(|x| *x != '\n')
        .take((c * r) as _)
        .map(|x: char| dsb::Cell {
            style: Style {
                bg: [255; 3],
                fg: [132, 148, 164],
                flags: 0,
                ..Default::default()
            },
            letter: Some(x),
        })
        .collect::<Vec<_>>();
    let now = Instant::now();

    let x = unsafe {
        dsb::render_owned(
            &cells,
            (c, r),
            ppem,
            &mut Fonts::new(*FONT, *FONT, *FONT, *FONT),
            lh,
            true,
        )
    };
    assert_eq!(include_bytes!("res"), x.bytes());
    std::hint::black_box(x);

    // println!("{:?}", now.elapsed());
    // x.as_ref().show();
}
pub static FONT: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::from_index(
        &include_bytes!("/home/os/CascadiaCodeNF.ttf")[..],
        0,
    )
    .unwrap()
});
